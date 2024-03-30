use bevy::{
    core_pipeline::{
        fxaa::{Fxaa, Sensitivity},
        prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
    },
    pbr::{DefaultOpaqueRendererMethod, ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
};
use bevy_mod_edge_detection::{EdgeDetectionConfig, EdgeDetectionPlugin, EdgeExtension};

fn main() {
    App::new()
        // MSAA currently doesn't work correctly with the plugin
        .insert_resource(Msaa::Off)
        .add_plugins((DefaultPlugins, EdgeDetectionPlugin))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, EdgeExtension>,
        >::default())
        .init_resource::<EdgeDetectionConfig>()
        // .init_resource::<DefaultOpaqueRendererMethod>()
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut extmaterials: ResMut<Assets<ExtendedMaterial<StandardMaterial, EdgeExtension>>>,
) {
    // set up the camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        // The edge detection effect requires the depth and normal prepass
        NormalPrepass,
        DepthPrepass,
        DeferredPrepass,
        // Add some anti-aliasing because the lines can be really harsh otherwise
        // This isn't required, but some form of AA is recommended
        Fxaa {
            enabled: true,
            edge_threshold: Sensitivity::Extreme,
            edge_threshold_min: Sensitivity::Extreme,
        },
        bevy_mod_edge_detection::EdgeDetectionCamera,
    ));

    // set up basic scene

    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0)),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });

    let mut stdmat: StandardMaterial = Color::rgb_u8(124, 144, 255).into();
    stdmat.opaque_render_method = OpaqueRendererMethod::Deferred;
    let extmat = ExtendedMaterial {
        base: stdmat,
        extension: EdgeExtension::default(),
    };
    // cube with extmat
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: extmaterials.add(extmat),
        transform: Transform::from_xyz(0.0, 1.6, 0.0),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::rgb_u8(124, 144, 255)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}
