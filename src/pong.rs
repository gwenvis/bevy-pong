use std::time::Duration;

use bevy::{PipelinedDefaultPlugins, app::prelude::*, asset::prelude::*, core::FixedTimestep, core::prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, ecs::prelude::*, ecs::schedule::ShouldRun, input::prelude::*, math::{Vec2, Vec3}, render2::{camera::OrthographicCameraBundle, color::Color, render_resource::{Extent3d, Texture, TextureFormat}, texture::Image, view::Visibility}, scene::prelude::*, sprite2::{*, self}, text::prelude::*, transform::prelude::*, window::prelude::*};
use rand::Rng;

const FRAMERATE: f64 = 60.0;
const TIMESTEP: f64 = 1.0 / FRAMERATE;

const PADDLE_OFFSET: f32 = 50.0;
const PADDLE_WIDTH: f32 = 12.5;
const PADDLE_HEIGHT: f32 = 800.0;//75.0;
const PADDLE_SPEED: f32 = 5.0 * (120.0 / FRAMERATE as f32);
const BOT_PADDLE_SPEED: f32 = 5.0 * (120.0 / FRAMERATE as f32);
const BALL_SIZE: f32 = 10.0;
const BALL_SPEED:f32 = 7.0 * (120.0 / FRAMERATE as f32);
const BALL_LAUNCH_TIME:f32 = 10.0;
const BALLS_AMOUNT:i64 = 100000;

pub fn run() {
    App::new()
        .add_event::<ScoreEvent>()
        .add_event::<ExitScreenEvent>()
        .add_startup_system(setup.system())
        .add_startup_stage("game_setup", 
        SystemStage::parallel()
                .with_system(spawn_paddles.system())
                .with_system(spawn_background.system())
            )
        .add_system_set(SystemSet::new()
            .with_run_criteria(FixedTimestep::step(TIMESTEP))
            .with_system(update_velocity.system().label("movement"))
            .with_system(ball_bounce.system().label("score").after("movement"))
            .with_system(remove_off_screen_balls.system().after("score"))
            .with_system(update_score.system().after("score")).label("physics"))
        .add_system_set(SystemSet::new()
            .with_run_criteria(should_spawn_balls.system())
            .with_system(spawn_ball.system()))
        .add_system_set(SystemSet::new()
            .with_run_criteria(should_launch_ball.system())
            .with_system(launch_ball.system()))
        .add_system(player_input.system())
        .add_system(paddle_boundaries.system())
        .add_system(bot_ai.system())
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(bevy::core_pipeline::ClearColor(Color::rgb(0.1, 0.1, 0.1)))
        .run();
}

struct Player;
struct Paddle;
struct Bot;
struct Ball;
struct Velocity(Vec2);

#[derive(Default)]
struct UiFont(Handle<Font>);

struct PlayerText();
struct OpponentText();
struct Score(Who, i32);
struct LaunchTimer(Timer);
struct BallCount(i32);
struct BallSprite(PipelinedSpriteBundle);

#[derive(PartialEq)]
enum Who { PLAYER, OPPONENT }

struct ScoreEvent(Who);
struct ExitScreenEvent(Entity, Who); 
struct PixelTexture(Texture);

fn setup(
    mut commands: Commands, 
    mut textures: ResMut<Assets<Image>>,
    asset_server : Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let image: Handle<Image> = asset_server.load("pixel.png");

    let font: Handle<Font> = asset_server.load("Consola.ttf");
    commands.insert_resource(UiFont(font));
    commands.insert_resource(BallCount(Default::default()));
    commands.insert_resource(LaunchTimer(Timer::new(Duration::from_secs_f32(BALL_LAUNCH_TIME), false)));
    commands.insert_resource(BallSprite(PipelinedSpriteBundle {
                sprite: sprite2::Sprite {
                    color : Color::WHITE,
                    custom_size: Some(Vec2::new(BALL_SIZE, BALL_SIZE)),
                    ..Default::default()
                },
                texture: image,
                visibility: Visibility { is_visible: true },
                ..Default::default()
            }));
}

fn spawn_background(
    mut commands: Commands,
    material: Res<BallSprite>,
    font: Res<UiFont>,
    windows: Res<Windows>
) {
    let window = windows.get_primary().unwrap();

    let mut mat = material.0.clone();
    mat.sprite.custom_size = Some(Vec2::new(2., window.height()));

    commands.spawn_bundle(mat);

    let text_y = window.height() / 2. * -1.;
    let text_x = window.width() / 4.;
    add_text(&mut commands, Vec2::new(text_x, text_y), &font, Who::PLAYER, PlayerText);
    add_text(&mut commands, Vec2::new(-text_x, text_y), &font, Who::OPPONENT, OpponentText);
}

fn should_launch_ball(
    mut timer: ResMut<LaunchTimer>,
    time : Res<Time>,
) -> ShouldRun {
    match timer.0.tick(time.delta()).just_finished() {
        true => ShouldRun::Yes,
        false => ShouldRun::No
    }
}

fn launch_ball(
    mut ball: Query<&mut Velocity, With<Ball>>
) {
    let mut random = rand::thread_rng();
    for mut b in ball.iter_mut() {
        let x = (random.gen::<f32>() - 0.5) * 2.;
        let y = random.gen::<f32>() - 0.5;

        b.0 = Vec2::new(x,y).normalize() * BALL_SPEED;
    }
}

fn bot_ai(
    mut bot_query : Query<(&Transform, &mut Velocity), With<Bot>>,
    ball_query : Query<&Transform, With<Ball>>
) {

    // Get the closest ball to the paddle
    for (t, mut v) in bot_query.iter_mut() {
        let mut ball : Vec3 = Vec3::ONE * f32::MAX;
        let mut dist = f32::MAX;
        for b in ball_query.iter() {
            let b_dist = (b.translation - t.translation).length();
            if b_dist < dist {
                ball = b.translation;
                dist = b_dist;
            }
        }

        let delta = ball.y - t.translation.y;
        let sign = delta.signum();
        v.0.y = f32::min(delta.abs(), BOT_PADDLE_SPEED) * sign;
    }
}

fn add_text(
    commands: &mut Commands,
    pos: Vec2,
    font: &Res<UiFont>,
    who: Who,
    component: impl bevy::ecs::component::Component
) {
    commands.spawn_bundle(Text2dBundle {
        text: Text::with_section(
            "0", TextStyle {
                font: font.0.clone(),
                font_size: 100.0,
                color: bevy::render::color::Color::WHITE,
            }, Default::default()),
        transform: Transform::from_xyz(pos.x, pos.y, 0.),
        ..Default::default()
    })
        .insert(Score(who, 0))
        .insert(component);
}

fn player_input(
    input : Res<Input<KeyCode>>,
    mut velocity: Query<&mut Velocity, With<Player>>
) {
    const SPEED:f32 = PADDLE_SPEED;

    for mut t in velocity.iter_mut() {
        if input.pressed(KeyCode::S) {
            t.0.y = -SPEED;
        } else if input.pressed(KeyCode::W) {
            t.0.y = SPEED;
        }
        else {
            t.0.y = 0.;
        }
    }
}

fn paddle_boundaries(
    mut transform: Query<&mut Transform, With<Paddle>>,
    windows : Res<Windows>
) {
    let window = windows.get_primary().unwrap();
    let height = window.height() / 2.;
    for mut t in transform.iter_mut() {
        if t.translation.y + PADDLE_HEIGHT / 2.0 > height {
            t.translation.y = height - PADDLE_HEIGHT / 2.0;
        }
        else if t.translation.y - PADDLE_HEIGHT / 2. < -height {
            t.translation.y = -height + PADDLE_HEIGHT / 2.;
        }
    }
}

fn ball_bounce(
    mut transform: Query<(&mut Velocity, &Transform, Entity), With<Ball>>, 
    paddles : Query<&Transform, With<Paddle>>,
    windows : Res<Windows>,
    mut bounce_event : EventWriter<ExitScreenEvent>,
) {
    let window = windows.get_primary().unwrap();
    let height = window.height() / 2.;
    let width = window.width() / 2.;

    for (mut v, t, e) in transform.iter_mut() {
        if t.translation.y + BALL_SIZE / 2. > height 
            || t.translation.y - BALL_SIZE / 2. < -height {
            v.0.y *= -1.;
        }

        if t.translation.x + BALL_SIZE / 2. > width
            || t.translation.x - BALL_SIZE / 2. < -width {
            bounce_event.send(ExitScreenEvent(e, if t.translation.x < 0. { Who::PLAYER } else { Who::OPPONENT }));
        }

        for pt in paddles.iter() {
            if t.translation.x - BALL_SIZE / 2. < pt.translation.x + PADDLE_WIDTH / 2. 
                && t.translation.x + BALL_SIZE / 2. > pt.translation.x - PADDLE_WIDTH / 2.
                && t.translation.y - BALL_SIZE / 2. < pt.translation.y + PADDLE_HEIGHT / 2.
                && t.translation.y + BALL_SIZE / 2. > pt.translation.y - PADDLE_HEIGHT / 2. {
                    //v.0.x *= -1.;
                    let bounce_vector = t.translation - pt.translation;
                    v.0 = (bounce_vector.normalize() * BALL_SPEED).truncate();
                }
        }
    }
}

fn spawn_paddles(mut commands: Commands, 
        mat : Res<BallSprite>,
        windows : Res<Windows>
) {
    let window = windows.get_primary().unwrap();
    let window_width_half: f32 = window.width() / 2.0;
    
    let mut clonedSprite = mat.0.clone();
    clonedSprite.sprite.custom_size = Some(Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT));
    clonedSprite.transform = Transform::from_xyz(-window_width_half + PADDLE_OFFSET, 0., 0.0);

    // spawn player
    commands.spawn()
        .insert_bundle(clonedSprite)
        .insert(Velocity(Default::default()))
        .insert(Player)
        .insert(Paddle);
    
    let mut oponnentSprite = mat.0.clone();
    oponnentSprite.sprite.custom_size = Some(Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT));
    oponnentSprite.transform = Transform::from_xyz(window_width_half - PADDLE_OFFSET, 0., 0.0);

    // spawn bot
    commands.spawn()
        .insert_bundle(oponnentSprite)
        .insert(Velocity(Default::default()))
        .insert(Bot)
        .insert(Paddle);
}

fn spawn_ball(
    mut commands: Commands, 
    mut ball_count : ResMut<BallCount>,
    mut timer : ResMut<LaunchTimer>,
    ball_sprite : Res<BallSprite>,
) {
    for _i in 0..BALLS_AMOUNT {
        commands
            .spawn()
            .insert_bundle(ball_sprite.0.clone())
            .insert(Velocity(Default::default()))
            .insert(Ball);
    }

    timer.0.reset();
    ball_count.0 = BALLS_AMOUNT as i32;
}

fn update_velocity(
    mut velocity : Query<(&Velocity, &mut Transform)>, 
) {
    for (v, mut t) in velocity.iter_mut() {
        t.translation += v.0.extend(0.);
    }
}

fn update_score(
    mut exit_screen_event : EventReader<ExitScreenEvent>,
    mut score_event : EventWriter<ScoreEvent>,
    mut scores : Query<(&mut Text, &mut Score)>,
) {

    fn update_text(text: &mut Text, score : i16) {
        text.sections[0].value = score.to_string();
    }

    for e in exit_screen_event.iter() {

        let result:Who = match e.1 {
            Who::PLAYER => Who::OPPONENT,
            Who::OPPONENT => Who::PLAYER
        };

        for (mut t, mut s) in scores.iter_mut() {
            if s.0 == e.1 {
                s.1 = s.1 + 1;
                update_text(&mut t, s.1.try_into().unwrap_or_default());
            }
        }

        score_event.send(ScoreEvent(result));
    }
}

fn remove_off_screen_balls(
    mut exit_screen_event : EventReader<ExitScreenEvent>,
    mut commands : Commands,
    mut ball_count : ResMut<BallCount>,
) {
    for e in exit_screen_event.iter() {
        commands.entity(e.0).despawn();
        ball_count.0 -= 1;
    }
}

fn should_spawn_balls(
    ball_count : Res<BallCount>
) -> ShouldRun {
    if ball_count.0 == 0 { ShouldRun::Yes }
    else { ShouldRun::No }
}