use bevy::{
    core_pipeline::{
        fxaa::{Fxaa, Sensitivity},
        prepass::{DeferredPrepass, DepthPrepass, NormalPrepass},
    },
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
};
use bevy_mod_edge_detection::{EdgeDetectionConfig, EdgeDetectionMaterial, EdgeDetectionPlugin};

fn main() {
    App::new()
        // MSAA currently doesn't work correctly with the plugin
        .insert_resource(Msaa::Off)
        .add_plugins((DefaultPlugins, EdgeDetectionPlugin))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, EdgeDetectionMaterial>,
        >::default())
        .init_resource::<EdgeDetectionConfig>()
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_things, keyboard_configuration_changing))
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut extmaterials: ResMut<Assets<ExtendedMaterial<StandardMaterial, EdgeDetectionMaterial>>>,
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
        extension: EdgeDetectionMaterial::default(),
    };
    // cube with extmat
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: extmaterials.add(extmat),
            transform: Transform::from_xyz(0.0, 2., 0.0),
            ..default()
        },
        Rotate,
    ));
    // cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::rgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(0.0, 0.7, 0.0),
            ..default()
        },
        Rotate,
    ));
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

#[derive(Component)]
struct Rotate;

fn rotate_things(mut query: Query<&mut Transform, With<Rotate>>) {
    for mut t in query.iter_mut() {
        t.rotate_x(0.01);
        t.rotate_y(0.005);
    }
}
fn keyboard_configuration_changing(
    button: Res<ButtonInput<KeyCode>>,
    mut conf: ResMut<EdgeDetectionConfig>,
) {
    if button.just_pressed(KeyCode::Space) {
        if conf.full_screen > 0 {
            conf.full_screen = 0;
        } else {
            conf.full_screen = 1;
        }
    }
    if button.just_pressed(KeyCode::KeyA) {
        conf.thickness += 0.1;
    }

    if button.just_pressed(KeyCode::KeyS) {
        conf.thickness -= 0.1;
    }
}
