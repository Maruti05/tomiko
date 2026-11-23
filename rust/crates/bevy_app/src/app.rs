use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Duration;

use audio_engine::{AudioCommand, AudioMeters};
use bevy::animation::graph::{AnimationGraph, AnimationNodeIndex};
use bevy::asset::AssetMetaCheck;
use bevy::camera::RenderTarget;
use bevy::gltf::Gltf;
use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::{BorderRadius, IsDefaultUiCamera, widget::NodeImageMode};
use bevy::ui_widgets::{Activate, Button, observe};
use bevy::window::{MonitorSelection, WindowMode};
use bevy::winit::WinitSettings;
use crossbeam_channel::Receiver;
use shared_types::{EngineState, VoicePreset};

use android_bridge::{EngineCommand, EngineHandle};

const CHARACTER_GLB: &str = "characters/tomiko/character.glb";
const BLINK_INDEX: usize = 14;
const VISEME_AA: usize = 0;
const VISEME_E: usize = 1;
const VISEME_O: usize = 3;
// The 3D stage renders off-screen at a lower resolution and is upscaled by
// the UI camera. This keeps low-end Adreno devices near 60 FPS while the HUD
// and text remain at native resolution.
const SCENE_WIDTH: u32 = 480;
const SCENE_HEIGHT: u32 = 1067;

const CLIP_NAMES: [&str; 11] = [
    "idle",
    "listening",
    "talking",
    "happy",
    "surprised",
    "sad",
    "angry",
    "sleeping",
    "blink",
    "touch_head",
    "touch_body",
];

pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    level: Level::INFO,
                    filter: "wgpu=error,naga=warn,bevy_render=info,bevy_ecs=warn".to_string(),
                    ..default()
                }),
        )
        .insert_resource(WinitSettings::mobile())
        .add_plugins(TomikoPlugin)
        .run();
}

struct TomikoPlugin;

impl Plugin for TomikoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (setup_scene, setup_audio, setup_engine_commands, setup_hud),
        )
        .add_systems(
            Update,
            (
                    spawn_character_when_loaded,
                    attach_animation_graph.run_if(resource_exists::<CharacterAnimations>),
                    apply_desired_clip.run_if(resource_exists::<CharacterAnimations>),
                    tick_one_shot,
                    tick_blink,
                    handle_touch,
                    handle_debug_keys,
                    drive_face_morphs,
                    drain_engine_commands,
                    sync_audio_state,
                    update_hud,
                ),
            );
    }
}

#[derive(Resource)]
struct CharacterAsset(Handle<Gltf>);

#[derive(Resource)]
struct CharacterAnimations {
    graph: Handle<AnimationGraph>,
    nodes: HashMap<&'static str, AnimationNodeIndex>,
}

#[derive(Resource)]
struct DesiredClip {
    name: &'static str,
    looping: bool,
}

impl Default for DesiredClip {
    fn default() -> Self {
        Self {
            name: "idle",
            looping: true,
        }
    }
}

#[derive(Resource, Default)]
struct AppliedClip {
    name: &'static str,
}

#[derive(Resource, Default)]
struct OneShot(Option<Timer>);

#[derive(Resource)]
struct BlinkTimer(Timer);

#[derive(Resource, Default)]
struct FaceState {
    blink_phase: f32,
    blink_active: bool,
}

#[derive(Resource)]
struct AudioState {
    meters: Arc<AudioMeters>,
    listening: bool,
}

#[derive(Resource, Default)]
struct TomikoUi {
    preset_index: usize,
    fps: f32,
    log_elapsed: f32,
    text_elapsed: f32,
}

#[derive(Component, Clone, Copy)]
enum HudLabel {
    Status,
    Mic,
    Replay,
    Preset,
}

#[derive(Component)]
struct CharacterRoot;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(ClearColor(Color::srgb(0.035, 0.040, 0.055)));
    commands.insert_resource(DesiredClip::default());
    commands.insert_resource(AppliedClip::default());
    commands.insert_resource(OneShot::default());
    commands.insert_resource(BlinkTimer(Timer::from_seconds(4.5, TimerMode::Repeating)));
    commands.insert_resource(FaceState::default());

    let scene_image = images.add(Image::new_target_texture(
        SCENE_WIDTH,
        SCENE_HEIGHT,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: Color::srgb(0.035, 0.040, 0.055).into(),
            ..default()
        },
        RenderTarget::Image(scene_image.clone().into()),
        Transform::from_xyz(0.0, 1.10, 4.20).looking_at(Vec3::new(0.0, 0.95, 0.0), Vec3::Y),
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: Color::srgb(0.02, 0.025, 0.035).into(),
            ..default()
        },
        IsDefaultUiCamera,
    ));

    commands.spawn((
        ImageNode {
            image: scene_image,
            image_mode: NodeImageMode::Stretch,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ));

    // Key light and a cooler fill. Shadows stay off on Android until device
    // validation; the first release uses a contact-shadow decal instead.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.88),
            illuminance: 11_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(3.0, 6.0, 4.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.55, 0.68, 1.0),
            illuminance: 3_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-4.0, 2.5, 2.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.06, 0.075, 0.10),
            perceptual_roughness: 0.92,
            ..default()
        })),
    ));

    // Backdrop.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 9.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.09, 0.10, 0.16),
            perceptual_roughness: 0.85,
            ..default()
        })),
        Transform::from_xyz(0.0, 3.2, -3.0).with_rotation(Quat::from_rotation_x(PI / 2.0)),
    ));

    let gltf: Handle<Gltf> = asset_server.load(CHARACTER_GLB);
    commands.insert_resource(CharacterAsset(gltf));
}

fn setup_audio(mut commands: Commands) {
    let meters = android_bridge::ensure_audio_runtime().unwrap_or_else(|error| {
        error!("Audio runtime unavailable: {error}");
        Arc::new(AudioMeters::default())
    });
    commands.insert_resource(AudioState {
        meters,
        listening: false,
    });
    info!("Tomiko audio runtime attached");
}

fn setup_hud(mut commands: Commands) {
    commands.insert_resource(TomikoUi::default());

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                height: Val::Px(150.0),
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.55)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Ready"),
                TextFont {
                    font_size: FontSize::Px(18.0),
                    ..default()
                },
                TextColor(Color::srgb(0.80, 0.88, 1.0)),
                HudLabel::Status,
            ));

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        hud_button(),
                        observe(
                            |_: On<Activate>, mut audio: ResMut<AudioState>| {
                                audio.listening = !audio.listening;
                                let command = if audio.listening {
                                    AudioCommand::StartListening
                                } else {
                                    AudioCommand::StopListening
                                };
                                if let Err(error) = android_bridge::send_audio(command) {
                                    warn!("Audio command failed: {error}");
                                }
                            },
                        ),
                        children![(hud_text("Talk"), HudLabel::Mic)],
                    ));

                    row.spawn((
                        hud_button(),
                        observe(|_: On<Activate>| {
                            if let Err(error) =
                                android_bridge::send_audio(AudioCommand::PlayLast)
                            {
                                warn!("Replay failed: {error}");
                            }
                        }),
                        children![(hud_text("Replay"), HudLabel::Replay)],
                    ));

                    row.spawn((
                        hud_button(),
                        observe(|_: On<Activate>, mut ui: ResMut<TomikoUi>| {
                            let presets = VoicePreset::ALL;
                            ui.preset_index = (ui.preset_index + 1) % presets.len();
                            let preset = presets[ui.preset_index];
                            if let Err(error) = android_bridge::send_audio(
                                AudioCommand::SetPreset {
                                    preset,
                                    intensity: 1.0,
                                },
                            ) {
                                warn!("Voice preset failed: {error}");
                            }
                        }),
                        children![(hud_text("Voice"), HudLabel::Preset)],
                    ));
                });
        });
}

fn hud_button() -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(118.0),
            height: Val::Px(52.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(Color::srgba(0.10, 0.13, 0.20, 0.92)),
        BorderColor::all(Color::srgba(0.35, 0.55, 0.85, 0.65)),
    )
}

fn hud_text(value: &str) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
    )
}

fn update_hud(
    time: Res<Time>,
    audio: Res<AudioState>,
    mut ui: ResMut<TomikoUi>,
    mut labels: Query<(&HudLabel, &mut Text)>,
) {
    let delta = time.delta_secs();
    if delta > 0.0 {
        let instant = 1.0 / delta;
        ui.fps = if ui.fps <= 0.0 {
            instant
        } else {
            ui.fps * 0.95 + instant * 0.05
        };
    }
    ui.log_elapsed += delta;
    if ui.log_elapsed >= 5.0 {
        info!("Render FPS: {:.1}", ui.fps);
        ui.log_elapsed = 0.0;
    }

    // Text layout is comparatively expensive; update the HUD at 4 Hz instead
    // of every frame so the render loop stays at the display refresh rate.
    ui.text_elapsed += delta;
    if ui.text_elapsed < 0.25 {
        return;
    }
    ui.text_elapsed = 0.0;

    let phase = match audio.meters.phase() {
        EngineState::Listening | EngineState::Capturing => "Listening",
        EngineState::Processing => "Thinking",
        EngineState::Playing => "Talking",
        EngineState::Paused => "Paused",
        EngineState::Error => "Mic error",
        EngineState::Idle => "Ready",
    };
    let presets = VoicePreset::ALL;
    let preset = presets[ui.preset_index.min(presets.len() - 1)];

    for (label, mut text) in &mut labels {
        let value = match label {
            HudLabel::Status => format!("{phase}  |  {}", preset.display_name()),
            HudLabel::Mic => {
                if audio.listening {
                    "Stop".to_owned()
                } else {
                    "Talk".to_owned()
                }
            }
            HudLabel::Replay => "Replay".to_owned(),
            HudLabel::Preset => preset.display_name().to_owned(),
        };
        if text.as_str() != value.as_str() {
            **text = value;
        }
    }
}

fn spawn_character_when_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset: Res<CharacterAsset>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    existing: Option<Res<CharacterAnimations>>,
) {
    if existing.is_some() {
        return;
    }
    if !asset_server.is_loaded_with_dependencies(&asset.0) {
        return;
    }
    let Some(gltf) = gltfs.get(&asset.0) else {
        return;
    };

    let mut names = Vec::new();
    let mut clips = Vec::new();
    for name in CLIP_NAMES {
        if let Some(clip) = gltf.named_animations.get(name) {
            names.push(name);
            clips.push(clip.clone());
        } else {
            warn!("Character clip missing: {name}");
        }
    }

    if clips.is_empty() {
        error!("Character has no usable animation clips");
        return;
    }

    let (graph, node_indices) = AnimationGraph::from_clips(clips);
    let mut nodes = HashMap::new();
    for (name, index) in names.into_iter().zip(node_indices) {
        nodes.insert(name, index);
    }

    let graph_handle = graphs.add(graph);
    commands.insert_resource(CharacterAnimations {
        graph: graph_handle,
        nodes,
    });

    let Some(scene) = gltf
        .default_scene
        .clone()
        .or_else(|| gltf.scenes.first().cloned())
    else {
        error!("Character GLB has no scene");
        return;
    };

    commands.spawn((
        WorldAssetRoot(scene),
        Transform::from_xyz(0.0, 0.0, 0.0),
        CharacterRoot,
    ));
    info!("Tomiko character scene spawned");
}

fn attach_animation_graph(
    mut commands: Commands,
    animations: Res<CharacterAnimations>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<AnimationGraphHandle>>,
) {
    for (entity, mut player) in &mut players {
        let mut transitions = AnimationTransitions::new();
        if let Some(idle) = animations.nodes.get("idle") {
            transitions
                .play(&mut player, *idle, Duration::ZERO)
                .repeat();
        }
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph.clone()))
            .insert(transitions);
        info!("Animation graph attached to player entity {entity:?}");
    }
}

fn apply_desired_clip(
    animations: Res<CharacterAnimations>,
    desired: Res<DesiredClip>,
    mut applied: ResMut<AppliedClip>,
    mut one_shot: ResMut<OneShot>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    if desired.name == applied.name {
        return;
    }
    let Some(node) = animations.nodes.get(desired.name) else {
        warn!("No animation node for clip {}", desired.name);
        return;
    };

    let mut played = false;
    for (mut player, mut transitions) in &mut players {
        let active = transitions.play(&mut player, *node, Duration::from_millis(180));
        if desired.looping {
            active.repeat();
        }
        played = true;
    }

    if !played {
        return;
    }

    if !desired.looping {
        let seconds = match desired.name {
            "blink" => 0.5,
            "touch_head" => 0.85,
            "touch_body" => 1.0,
            "happy" => 1.4,
            "surprised" => 0.8,
            "sad" => 1.4,
            "angry" => 1.1,
            _ => 1.2,
        };
        one_shot.0 = Some(Timer::from_seconds(seconds, TimerMode::Once));
    } else {
        one_shot.0 = None;
    }

    applied.name = desired.name;
    info!("Playing clip {}", desired.name);
}

fn tick_one_shot(time: Res<Time>, mut one_shot: ResMut<OneShot>, mut desired: ResMut<DesiredClip>) {
    let Some(timer) = one_shot.0.as_mut() else {
        return;
    };
    timer.tick(time.delta());
    if timer.just_finished() {
        one_shot.0 = None;
        desired.name = "idle";
        desired.looping = true;
    }
}

fn tick_blink(
    time: Res<Time>,
    mut timer: ResMut<BlinkTimer>,
    mut desired: ResMut<DesiredClip>,
    applied: Res<AppliedClip>,
    audio: Res<AudioState>,
) {
    timer.0.tick(time.delta());
    if audio.meters.phase() == EngineState::Playing {
        return;
    }
    if timer.0.just_finished() && desired.name == "idle" && applied.name == "idle" {
        desired.name = "blink";
        desired.looping = false;
    }
}

fn handle_touch(
    mut touches: MessageReader<TouchInput>,
    keys: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut desired: ResMut<DesiredClip>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let height = window.height().max(1.0);

    let mut position = None;
    for touch in touches.read() {
        if touch.phase == TouchPhase::Started {
            position = Some(touch.position);
        }
    }
    if position.is_none() && keys.just_pressed(MouseButton::Left) {
        position = window.cursor_position();
    }

    let Some(position) = position else {
        return;
    };

    // Window coordinates grow downwards, so the top of the screen is the head.
    desired.name = if position.y < height * 0.55 {
        "touch_head"
    } else {
        "touch_body"
    };
    desired.looping = false;
}

/// Debug/desktop shortcuts. On a device these can be triggered with
/// `adb shell input keyevent <keycode>` (for example 48 = T).
///
/// Audio: Space (62) toggles listening, P (44) replays, Y (53) runs the
/// synthetic DSP/playback self-test.
fn handle_debug_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut desired: ResMut<DesiredClip>,
    mut audio: ResMut<AudioState>,
) {
    if keys.just_pressed(KeyCode::Space) {
        audio.listening = !audio.listening;
        let command = if audio.listening {
            AudioCommand::StartListening
        } else {
            AudioCommand::StopListening
        };
        if let Err(error) = android_bridge::send_audio(command) {
            warn!("Audio command failed: {error}");
        }
    }
    if keys.just_pressed(KeyCode::KeyP)
        && let Err(error) = android_bridge::send_audio(AudioCommand::PlayLast)
    {
        warn!("PlayLast failed: {error}");
    }
    if keys.just_pressed(KeyCode::KeyY)
        && let Err(error) = android_bridge::send_audio(AudioCommand::SelfTest)
    {
        warn!("Self-test failed: {error}");
    }
    if keys.just_pressed(KeyCode::KeyG)
        && let Err(error) = android_bridge::send_audio(AudioCommand::SelfTestCapture)
    {
        warn!("VAD self-test failed: {error}");
    }

    let bindings = [
        (KeyCode::KeyI, "idle", true),
        (KeyCode::KeyL, "listening", true),
        (KeyCode::KeyT, "talking", true),
        (KeyCode::KeyH, "happy", false),
        (KeyCode::KeyU, "surprised", false),
        (KeyCode::KeyS, "sad", false),
        (KeyCode::KeyA, "angry", false),
        (KeyCode::KeyZ, "sleeping", true),
        (KeyCode::KeyB, "blink", false),
        (KeyCode::KeyQ, "touch_head", false),
        (KeyCode::KeyW, "touch_body", false),
    ];
    for (key, name, looping) in bindings {
        if keys.just_pressed(key) {
            desired.name = name;
            desired.looping = looping;
        }
    }
}

fn drive_face_morphs(
    time: Res<Time>,
    desired: Res<DesiredClip>,
    mut face: ResMut<FaceState>,
    audio: Res<AudioState>,
    mut morphs: Query<&mut MorphWeights>,
) {
    let delta = time.delta_secs();

    if desired.name == "blink" {
        face.blink_active = true;
        face.blink_phase += delta;
    } else {
        face.blink_active = false;
        face.blink_phase = 0.0;
    }

    let mouth_open = audio.meters.mouth_open();
    let viseme_aa = audio.meters.viseme_aa();
    let viseme_e = audio.meters.viseme_e();
    let viseme_o = audio.meters.viseme_o();

    for mut weights in &mut morphs {
        let weights = weights.weights_mut();
        if weights.len() < 15 {
            continue;
        }
        weights.fill(0.0);

        if face.blink_active {
            // One blink cycle: 0 -> 1 -> 0 over ~0.5 s.
            let t = (face.blink_phase / 0.5).clamp(0.0, 1.0);
            weights[BLINK_INDEX] = (t * PI).sin();
        }

        if mouth_open > 0.001 {
            weights[VISEME_AA] = viseme_aa;
            weights[VISEME_E] = viseme_e;
            weights[VISEME_O] = viseme_o;
        }
    }
}

#[derive(Resource, Debug)]
struct EngineCommands(Receiver<EngineCommand>);

fn setup_engine_commands(mut commands: Commands) {
    if let Some(receiver) = android_bridge::take_receiver() {
        commands.insert_resource(EngineCommands(receiver));
        info!("Tomiko native command receiver attached");
    } else {
        warn!("Tomiko native command receiver was already taken");
    }
}

fn drain_engine_commands(
    commands: Res<EngineCommands>,
    mut desired: ResMut<DesiredClip>,
    mut players: Query<&mut AnimationPlayer>,
    mut audio: ResMut<AudioState>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(handle) = android_bridge::global() else {
        return;
    };

    while let Ok(command) = commands.0.try_recv() {
        info!("Native command: {command:?}");
        apply_engine_state(handle, &command);

        match &command {
            EngineCommand::StartListening => audio.listening = true,
            EngineCommand::StopListening | EngineCommand::Shutdown => audio.listening = false,
            _ => {}
        }

        match command {
            EngineCommand::StartListening => {
                desired.name = "listening";
                desired.looping = true;
            }
            EngineCommand::StopListening => {
                desired.name = "idle";
                desired.looping = true;
            }
            EngineCommand::Shutdown => {
                desired.name = "idle";
                desired.looping = true;
                // The GameActivity is going away; stop the Bevy loop instead
                // of spinning without a surface.
                exit.write(AppExit::Success);
            }
            EngineCommand::PlayLast => {
                desired.name = "talking";
                desired.looping = true;
            }
            EngineCommand::Pause => {
                for mut player in &mut players {
                    player.pause_all();
                }
            }
            EngineCommand::Resume => {
                for mut player in &mut players {
                    player.resume_all();
                }
            }
            EngineCommand::SetVoicePreset { .. }
            | EngineCommand::SetQuality(_)
            | EngineCommand::SetCharacter(_) => {}
        }
    }
}

fn sync_audio_state(audio: Res<AudioState>, mut desired: ResMut<DesiredClip>) {
    let phase = audio.meters.phase();

    // Only steer clips that the audio engine owns. One-shot debug/touch clips
    // keep playing until they finish, then fall back to the audio phase.
    if matches!(desired.name, "idle" | "listening" | "talking") {
        let (name, looping) = match phase {
            EngineState::Playing => ("talking", true),
            EngineState::Listening | EngineState::Capturing | EngineState::Processing => {
                ("listening", true)
            }
            EngineState::Idle | EngineState::Error => ("idle", true),
            EngineState::Paused => (desired.name, desired.looping),
        };
        if desired.name != name {
            desired.name = name;
            desired.looping = looping;
        }
    }

    if let Some(handle) = android_bridge::global() {
        use shared_types::CharacterState;

        handle.set_engine_state(phase);
        handle.set_amplitude(audio.meters.amplitude());
        handle.set_mouth_open(audio.meters.mouth_open());
        handle.set_error_code(audio.meters.error_code());
        match phase {
            EngineState::Playing => handle.set_character_state(CharacterState::Talking),
            EngineState::Listening | EngineState::Capturing | EngineState::Processing => {
                handle.set_character_state(CharacterState::Listening);
            }
            EngineState::Idle => handle.set_character_state(CharacterState::Idle),
            EngineState::Paused | EngineState::Error => {}
        }
    }
}

fn apply_engine_state(handle: &EngineHandle, command: &EngineCommand) {
    use shared_types::{CharacterState, EngineState};

    match command {
        EngineCommand::StartListening => {
            handle.set_engine_state(EngineState::Listening);
            handle.set_character_state(CharacterState::Listening);
        }
        EngineCommand::StopListening => {
            handle.set_engine_state(EngineState::Idle);
            handle.set_character_state(CharacterState::Idle);
        }
        EngineCommand::PlayLast => {
            handle.set_engine_state(EngineState::Playing);
            handle.set_character_state(CharacterState::Talking);
        }
        EngineCommand::SetVoicePreset { .. } | EngineCommand::SetCharacter(_) => {}
        EngineCommand::SetQuality(tier) => handle.set_quality_tier(*tier),
        EngineCommand::Pause => handle.set_engine_state(EngineState::Paused),
        EngineCommand::Resume => handle.set_engine_state(EngineState::Idle),
        EngineCommand::Shutdown => {
            handle.set_engine_state(EngineState::Idle);
            handle.set_character_state(CharacterState::Idle);
            handle.set_amplitude(0.0);
            handle.set_progress(0.0);
            handle.set_mouth_open(0.0);
        }
    }
}
