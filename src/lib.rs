use bevy::{
    asset::load_internal_asset,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
        prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
    },
    pbr::{DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_resource::{
            binding_types::{
                sampler, texture_2d, texture_depth_2d, uniform_buffer, uniform_buffer_sized,
            },
            AsBindGroup, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, FragmentState, MultisampleState, PipelineCache,
            PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderRef, ShaderStages, ShaderType, TextureFormat,
            TextureSampleType, UniformBuffer,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::ViewUniform,
        Extract, Render, RenderApp, RenderSet,
    },
};
use node::EdgeDetectionNode;

pub mod prelude {
    pub use crate::{
        traits::*, EdgeDetectionCamera, EdgeDetectionCameraMarkerBundle, EdgeDetectionConfig,
        EdgeDetectionMaterial, EdgeDetectionPlugin, StandardEdgeDetectionMaterial,
    };
}

use crate::node::EdgeDetetctionNodeLabel;

mod node;

pub const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(410592619790336);
pub const SHADER_MATERIAL_HANDLE: Handle<Shader> = Handle::weak_from_u128(410592619790337);

pub struct EdgeDetectionPlugin;
impl Plugin for EdgeDetectionPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER_HANDLE, "edge_detection.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            SHADER_MATERIAL_HANDLE,
            "edge_detection_material.wgsl",
            Shader::from_wgsl
        );
        // app.add_systems(Update, print_projection);

        app.init_resource::<DefaultOpaqueRendererMethod>();
        app.init_resource::<EdgeDetectionConfig>();
        app.add_plugins(ExtractComponentPlugin::<EdgeDetectionCamera>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(ExtractSchedule, extract_config)
            .add_systems(Render, prepare_config_buffer.in_set(RenderSet::Prepare));

        render_app
            .add_render_graph_node::<ViewNodeRunner<EdgeDetectionNode>>(
                Core3d,
                EdgeDetetctionNodeLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    EdgeDetetctionNodeLabel,
                    Node3d::Tonemapping,
                    // Node3d::DeferredPrepass, // adding this makes the app not start
                ),
            );
    }
    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<EdgeDetectionPipeline>()
            .init_resource::<ConfigBuffer>();
    }
}
#[derive(Component, Clone, Copy, ExtractComponent)]
pub struct EdgeDetectionCamera;

#[derive(Bundle)]
/// Marker components needed for the camera to run the edgedetection post-processing
/// The edge detection effect requires the depth, normal as deferred prepass, as well as a specific marker component
/// NOTE: they can all be added induvidially to the camera, in case some of the marker components are already present
pub struct EdgeDetectionCameraMarkerBundle {
    camera: EdgeDetectionCamera,
    depth_prepass: DepthPrepass,
    normal_prepass: NormalPrepass,
    deferred_prepass: DeferredPrepass,
}

#[derive(Resource, ShaderType, Clone, Copy)]
/// Determines how the edges are to be calculated, and how they will look
pub struct EdgeDetectionConfig {
    /// thickness of the edge calculations
    pub thickness: f32,
    pub depth_threshold: f32,
    pub normal_threshold: f32,
    pub color_threshold: f32,
    pub edge_color: Color,
    pub debug: u32,
    /// Determines if the edge detection should be for the entire screen,
    /// or only for entites with the correct material
    pub full_screen: u32,
}

impl Default for EdgeDetectionConfig {
    fn default() -> Self {
        Self {
            thickness: 0.8,
            depth_threshold: 0.2,
            normal_threshold: 0.05,
            color_threshold: 1.0,
            edge_color: Color::BLACK,
            debug: 0,
            full_screen: 1,
        }
    }
}

/// The Extended Material type for edge-detection post-processing
pub type EdgeDetectionMaterial<B> = ExtendedMaterial<B, EdgeDetectionMaterialExtension>;
pub type StandardEdgeDetectionMaterial = EdgeDetectionMaterial<StandardMaterial>;

pub mod traits {
    use bevy::pbr::{Material, OpaqueRendererMethod, StandardMaterial};

    use crate::{
        EdgeDetectionMaterial, EdgeDetectionMaterialExtension, StandardEdgeDetectionMaterial,
    };

    /// have trait to make it easier to generate the material from standard material
    pub trait ToEdgeMaterial: Material {
        fn to_edge_material(self) -> EdgeDetectionMaterial<Self> {
            EdgeDetectionMaterial {
                base: self,
                extension: EdgeDetectionMaterialExtension::default(),
            }
        }
    }

    impl ToEdgeMaterial for StandardMaterial {
        fn to_edge_material(mut self) -> EdgeDetectionMaterial<StandardMaterial> {
            self.opaque_render_method = OpaqueRendererMethod::Deferred;
            EdgeDetectionMaterial {
                base: self,
                extension: EdgeDetectionMaterialExtension::default(),
            }
        }
    }

    /// have trait to make it easier to generate the material from standard material
    pub trait FromEdgeMaterial<B: Material> {
        fn from_edge_material(self) -> B;
    }

    impl FromEdgeMaterial<StandardMaterial> for StandardEdgeDetectionMaterial {
        fn from_edge_material(self) -> StandardMaterial {
            self.base
        }
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
/// Material that will enable the postprocess Edge detection effect on a specific entity, instead of entire screen
pub struct EdgeDetectionMaterialExtension {
    #[uniform(100)]
    _phantom: u32,
}

impl Default for EdgeDetectionMaterialExtension {
    fn default() -> Self {
        Self { _phantom: 0 }
    }
}

impl MaterialExtension for EdgeDetectionMaterialExtension {
    /// this material will only work in in deferred rendering
    fn deferred_fragment_shader() -> ShaderRef {
        SHADER_MATERIAL_HANDLE.into()
    }
}

#[derive(Resource)]
struct ConfigBuffer {
    buffer: UniformBuffer<EdgeDetectionConfig>,
}

impl FromWorld for ConfigBuffer {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();

        let config = EdgeDetectionConfig::default();
        let mut buffer = UniformBuffer::default();
        buffer.set(config);
        buffer.write_buffer(render_device, render_queue);

        ConfigBuffer { buffer }
    }
}

fn extract_config(mut commands: Commands, config: Extract<Res<EdgeDetectionConfig>>) {
    commands.insert_resource(**config);
}

fn prepare_config_buffer(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut config_buffer: ResMut<ConfigBuffer>,
    config: Res<EdgeDetectionConfig>,
) {
    let buffer = config_buffer.buffer.get_mut();
    *buffer = *config;
    config_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
}

#[derive(Resource)]
struct EdgeDetectionPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for EdgeDetectionPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "edge_detection_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // screen_texture
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    // depth prepass
                    texture_depth_2d(),
                    // normal prepass
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // deferred_texture
                    texture_2d(TextureSampleType::Uint),
                    // view
                    uniform_buffer::<ViewUniform>(true),
                    // config
                    uniform_buffer_sized(false, None),
                    // // deferred prepass
                    // texture_2d(TextureSampleType::Float { filterable: true }),
                ),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("edge_detection_pipeline".into()),
                    layout: vec![layout.clone()],
                    // This will setup a fullscreen triangle for the vertex state
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: SHADER_HANDLE,
                        shader_defs: vec!["VIEW_PROJECTION_PERSPECTIVE".into()], // TODO detect projection
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                });

        Self {
            layout,
            sampler,
            pipeline_id,
        }
    }
}
