use std::ops::{Add, Div, Mul};
use bevy::{ecs::query::Has, prelude::*};
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_ecs::component::{SparseStorage, TableStorage};
use bevy_ecs::system::SystemParam;
use bevy_egui::{egui, EguiContexts};
use bevy_egui::egui::{Color32, Id, Pos2};
use bevy_persistent::Persistent;
use bevy_xpbd_3d::{math::*, prelude::*, SubstepSchedule, SubstepSet};
use bevy_xpbd_3d::parry::na::DimMul;
use leafwing_input_manager::orientation::Orientation;
use leafwing_input_manager::prelude::InputKind::Mouse;
use leafwing_input_manager::prelude::*;
use crate::building::DoorEnum;
use crate::{PlayerMarker, Settings};
use crate::states::{AppCursorState, MyAppState};

#[derive(PhysicsLayer, Clone, Copy, Debug)]
pub enum GameLayer {
    Player,
    // Layer 0
    Environment,
    RaycastInteractible,
}

pub trait EguiInteractableComponent: Component<Storage=SparseStorage> {
    type Param<'a, 'b>: SystemParam;
    fn system(param: Self::Param<'_, '_>);
}

pub trait EguiInteractableEmpty<EguiInteractableComponent: Component>: Component<Storage=TableStorage> {
    fn build_interactable_component(&self) -> EguiInteractableComponent;
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DoorEguiInteractableComponent;
#[derive(SystemParam)]
pub struct DoorEguiInteractionParam<'w, 's> {
    egui_contexts: EguiContexts<'w, 's>,
    q_door_egui_component: Query<'w, 's, (&'static mut DoorEguiInteractableEmpty,&'static mut Transform),(Without<Camera>, Without<CharacterController>,With<DoorEguiInteractableComponent>)>,
    q_player_transform: Query<'w, 's, &'static mut Transform, (With<Camera>, With<CharacterController>)>,
}

impl EguiInteractableComponent for DoorEguiInteractableComponent {
    type Param<'a, 'b> = DoorEguiInteractionParam<'a, 'b>;
    fn system(mut param: Self::Param<'_, '_>) {
        param.q_door_egui_component.iter_mut().for_each(|mut door_egui_interactable_component| {
            let id = door_egui_interactable_component.0.id.to_string();
            let door_enum = door_egui_interactable_component.0.door_enum;
            egui::Window::new("Door").id(Id::new(id)).open(&mut door_egui_interactable_component.0.window_open).show(param.egui_contexts.ctx_mut(), |ui| {
                if let DoorEnum::Interior(locked ) = door_enum{
                    ui.label("This door is locked.");
                }else {
                    if ui.button("enter").clicked(){
                    if(door_egui_interactable_component.1.rotation.y / (std::f32::consts::PI * 2.0) % 2.0 == 1.0){
                        let diff = param.q_player_transform.get_single_mut().unwrap().translation.x - door_egui_interactable_component.1.translation.x;
                        if diff > 0.0{
                            param.q_player_transform.get_single_mut().unwrap().translation.x -= (diff+1.0);
                        }else {
                            param.q_player_transform.get_single_mut().unwrap().translation.x -= (diff-1.0);
                        }
                        param.q_player_transform.get_single_mut().unwrap().translation.z = door_egui_interactable_component.1.translation.z;
                    }else{
                        let diff = param.q_player_transform.get_single_mut().unwrap().translation.z - door_egui_interactable_component.1.translation.z;
                        if diff > 0.0{
                            param.q_player_transform.get_single_mut().unwrap().translation.z -= (diff+1.0);
                        }else {
                            param.q_player_transform.get_single_mut().unwrap().translation.z -= (diff-1.0);
                        }
                        param.q_player_transform.get_single_mut().unwrap().translation.x = door_egui_interactable_component.1.translation.x;
                    }
                }
            }
            });
        });
    }
}

#[derive(Component)]
pub struct DoorEguiInteractableEmpty {
    pub(crate) id: String,
    pub(crate) window_open:bool,
    pub(crate) door_enum: DoorEnum,
}


impl EguiInteractableEmpty<DoorEguiInteractableComponent> for DoorEguiInteractableEmpty {
    fn build_interactable_component(&self) -> DoorEguiInteractableComponent {
        DoorEguiInteractableComponent {}
    }
}
#[derive(Debug, Default, Component)]
pub struct VirtualPointer {
    pub start_click_pos: Option<Vec2>,
    pub current_pos: Option<Vec2>,
}
pub struct CharacterControllerPlugin;
#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum PlayerMovement {
    Look,
    Move,
    Click,
    UIToggle,
    Pause,
}
impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Update,
                (
                    movement,
                    apply_movement_damping,
                    mesh_snip_select_system,
                    DoorEguiInteractableComponent::system.run_if(any_with_component::<DoorEguiInteractableComponent>)
                ).run_if(in_state(MyAppState::InGame))
                    .chain(),
            ).add_systems(Update,paused_update.run_if(in_state(MyAppState::Paused)))
            .add_systems(OnEnter(MyAppState::LoadingScreen),load)
            .add_plugins(InputManagerPlugin::<PlayerMovement>::default())
            .add_systems(
                // Run collision handling in substep schedule
                SubstepSchedule,
                kinematic_controller_collisions.in_set(SubstepSet::SolveUserConstraints),
            );
    }
}
fn load(mut commands: Commands,
         mut q_windows: Query<&mut Window, With<PrimaryWindow>>){
    let mut primary_window = q_windows.single_mut();
    primary_window.cursor.grab_mode = CursorGrabMode::Confined;
    primary_window.cursor.visible = false;
    commands.spawn((
        VirtualPointer { start_click_pos: None, current_pos: Some(Vec2 { x: primary_window.width() * 0.5, y: primary_window.height() * 0.5 }) },
    ));
}
/// An event sent for a movement input action

/// A marker component indicating that an entity is using a character controller.
#[derive(Component)]
pub struct CharacterController;

/// A marker component indicating that an entity is on the ground.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;
/// The acceleration used for character movement.
#[derive(Component)]
pub struct MovementAcceleration(Scalar);

/// The damping factor used for slowing down movement.
#[derive(Component)]
pub struct MovementDampingFactor(Scalar);

/// A bundle that contains the components needed for a basic
/// kinematic character controller.
#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    rigid_body: RigidBody,
    collider: Collider,
    movement: MovementBundle,
    collision_layers: CollisionLayers,
    input_manager:  InputManagerBundle<PlayerMovement>
}

/// A bundle that contains components for character movement.
#[derive(Bundle)]
pub struct MovementBundle {
    acceleration: MovementAcceleration,
    damping: MovementDampingFactor,
}

impl MovementBundle {
    pub const fn new(
        acceleration: Scalar,
        damping: Scalar,
    ) -> Self {
        Self {
            acceleration: MovementAcceleration(acceleration),
            damping: MovementDampingFactor(damping),
        }
    }
}

impl Default for MovementBundle {
    fn default() -> Self {
        Self::new(30.0, 0.9)
    }
}

impl CharacterControllerBundle {
    pub fn new(collider: Collider, input_manager: InputManagerBundle<PlayerMovement>) -> Self {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vector::ONE * 0.99, 10);
        Self {
            character_controller: CharacterController,
            rigid_body: RigidBody::Kinematic,
            collider,
            collision_layers:CollisionLayers::new(GameLayer::Player, [GameLayer::Environment, GameLayer::Player,GameLayer::RaycastInteractible]),
            movement: MovementBundle::default(),
            input_manager,
        }
    }

    pub fn with_movement(
        mut self,
        acceleration: Scalar,
        damping: Scalar,
    ) -> Self {
        self.movement = MovementBundle::new(acceleration, damping);
        self
    }
}

fn movement(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
            mut contexts: EguiContexts,
            mut next_app_state: ResMut<NextState<MyAppState>>,
            mut next_cursor_state: ResMut<NextState<AppCursorState>>,
            // state:Res<State<MyAppState>>,
            mut q_pointer: Query<&mut VirtualPointer, With<VirtualPointer>>,
            cursor_state: Res<State<AppCursorState>>,
            time: Res<Time>,
            settings: Res<Persistent<Settings>>,
    mut controllers: Query<(
        &ActionState<PlayerMovement>,
        &MovementAcceleration,
        &mut LinearVelocity,
        &mut Rotation,
        &mut Transform,
    )>,
) {
    for ( mut action_state, movement_acceleration, mut linear_velocity, mut rotation,mut transform) in
    &mut controllers
    {
        let mut primary_window = q_windows.single_mut();
        let mut pointer = q_pointer.get_single_mut().unwrap();
        if action_state.just_pressed(&PlayerMovement::Pause) {
            next_app_state.set(MyAppState::Paused);
            primary_window.cursor.grab_mode = CursorGrabMode::None;
            next_cursor_state.set(AppCursorState::Free);
            pointer.start_click_pos = None;
        }
        if action_state.just_pressed(&PlayerMovement::UIToggle) {
            next_cursor_state.set(AppCursorState::Virtual);
            primary_window.cursor.grab_mode = CursorGrabMode::Confined;
        } else if action_state.just_released(&PlayerMovement::UIToggle) {
            next_cursor_state.set(AppCursorState::Locked);
            primary_window.cursor.grab_mode = CursorGrabMode::Locked;
            pointer.start_click_pos = None;
        }
        if (cursor_state.get().ne(&AppCursorState::Free)) {
            primary_window.set_cursor_position(pointer.current_pos);
            if (cursor_state.get().ne(&AppCursorState::Virtual)) {
                pointer.current_pos = Some(Vec2 { x: primary_window.width() / 2.0, y: primary_window.height() / 2.0 });
                if action_state.pressed(&PlayerMovement::Look) {
                    let axis_pair = action_state.axis_pair(&PlayerMovement::Look).unwrap();
                    transform.rotate_local_y( axis_pair.x() * time.delta_seconds() / settings.look_sensitivity * -1.0);
                }
            } else {
                if action_state.pressed(&PlayerMovement::Look) {
                    if let Some(axis_pair) = action_state.axis_pair(&PlayerMovement::Look) {
                        pointer.current_pos = Some(pointer.current_pos.unwrap().add(axis_pair.xy().div(settings.mouse_sensitivity)).clamp(Vec2::ZERO, Vec2 { x: primary_window.width(), y: primary_window.height() }));
                    }
                }
            }
            if action_state.pressed(&PlayerMovement::Move) {
                let axis_pair = action_state.clamped_axis_pair(&PlayerMovement::Move).unwrap();
                let velocity =((transform.left() * axis_pair.x() * -1.0 )+( transform.forward() * axis_pair.y()))
                    *movement_acceleration.0 * time.delta().as_secs_f32() * crate::SPEED;
                linear_velocity.x = velocity.x ;
                linear_velocity.z = velocity.z ;

            }
        }
    }
}

// / Applies [`ControllerGravity`] to character controllers.

/// Slows down movement in the XZ plane.
fn apply_movement_damping(mut query: Query<(&MovementDampingFactor, &mut LinearVelocity)>) {
    for (damping_factor, mut linear_velocity) in &mut query {
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        linear_velocity.x *= damping_factor.0;
        linear_velocity.z *= damping_factor.0;
    }
}

/// Kinematic bodies do not get pushed by collisions by default,
/// so it needs to be done manually.
///
/// This system performs very basic collision response for kinematic
/// character controllers by pushing them along their contact normals
/// by the current penetration depths.
#[allow(clippy::type_complexity)]
fn kinematic_controller_collisions(
    collisions: Res<Collisions>,
    collider_parents: Query<&ColliderParent, Without<Sensor>>,
    mut character_controllers: Query<
        (
            &RigidBody,
            &mut Position,
            &Rotation,
            &mut LinearVelocity,
        ),
        With<CharacterController>,
    >,
) {
    // Iterate through collisions and move the kinematic body to resolve penetration
    for contacts in collisions.iter() {
        // If the collision didn't happen during this substep, skip the collision
        if !contacts.during_current_substep {
            continue;
        }

        // Get the rigid body entities of the colliders (colliders could be children)
        let Ok([collider_parent1, collider_parent2]) =
            collider_parents.get_many([contacts.entity1, contacts.entity2])
        else {
            continue;
        };

        // Get the body of the character controller and whether it is the first
        // or second entity in the collision.
        let is_first: bool;
        let (rb, mut position, rotation, mut linear_velocity) =
            if let Ok(character) = character_controllers.get_mut(collider_parent1.get()) {
                is_first = true;
                character
            } else if let Ok(character) = character_controllers.get_mut(collider_parent2.get()) {
                is_first = false;
                character
            } else {
                continue;
            };

        // This system only handles collision response for kinematic character controllers
        if !rb.is_kinematic() {
            continue;
        }

        // Iterate through contact manifolds and their contacts.
        // Each contact in a single manifold shares the same contact normal.
        for manifold in contacts.manifolds.iter() {
            let normal = if is_first {
                -manifold.global_normal1(rotation)
            } else {
                -manifold.global_normal2(rotation)
            };

            // Solve each penetrating contact in the manifold
            for contact in manifold.contacts.iter().filter(|c| c.penetration > 0.0) {
                position.0 += normal * contact.penetration;
            }

        }
    }
}
const RAY_RANGE: f32 = 5.0;
fn mesh_snip_select_system(
    mut commands: Commands,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut q_player: Query<(&ActionState<PlayerMovement>, &GlobalTransform, &Camera), (With<CharacterController>,With<Camera>)>,
    mut q_pointer: Query<&mut VirtualPointer, With<VirtualPointer>>,
    mut q_door_egui_interactable_empty: Query<&DoorEguiInteractableEmpty,(With<DoorEguiInteractableEmpty>,Without<DoorEguiInteractableComponent>)>,
    mut q_door_egui_interactable_component: Query<(&DoorEguiInteractableComponent,&GlobalTransform,Entity,&mut DoorEguiInteractableEmpty),With<DoorEguiInteractableComponent>>,
    mut egui_contexts: EguiContexts,
    mut gizmos: Gizmos,
    cursor_state: Res<State<AppCursorState>>,
    spatial_query: SpatialQuery) {
    let ctx = egui_contexts.ctx_mut();
    let (mut action_state, transform, camera) = q_player.single_mut(); // Cast ray and print first hit
    let mut pointer = q_pointer.get_single_mut().unwrap();
    for mut door in  &mut q_door_egui_interactable_component{
        if transform.translation().distance(door.1.translation()) > RAY_RANGE{
            commands.get_entity(door.2).unwrap().remove::<DoorEguiInteractableComponent>();
        }
        if(!door.3.window_open){
            commands.get_entity(door.2).unwrap().remove::<DoorEguiInteractableComponent>();
            door.3.window_open = true;
        }
    }

    if cursor_state.eq(&AppCursorState::Virtual) {
        if !ctx.is_using_pointer() && !ctx.is_pointer_over_area() {
            let end_pointer_pos = pointer.current_pos.unwrap();
            if action_state.just_pressed(&PlayerMovement::Click) {
                pointer.start_click_pos = pointer.current_pos;
            } else if let Some(start_pointer_pos) = pointer.start_click_pos {
                let Some(left_bottom) = camera.viewport_to_world(&transform, Vec2 { x: start_pointer_pos.x.min(end_pointer_pos.x), y: start_pointer_pos.y.min(end_pointer_pos.y) }) else {
                    return;
                };
                let Some(right_bottom) = camera.viewport_to_world(&transform, Vec2 { x: start_pointer_pos.x.max(end_pointer_pos.x), y: start_pointer_pos.y.min(end_pointer_pos.y) }) else {
                    return;
                };
                let Some(right_top) = camera.viewport_to_world(&transform, Vec2 { x: start_pointer_pos.x.max(end_pointer_pos.x), y: start_pointer_pos.y.max(end_pointer_pos.y) }) else {
                    return;
                };
                let Some(left_top) = camera.viewport_to_world(&transform, Vec2 { x: start_pointer_pos.x.min(end_pointer_pos.x), y: start_pointer_pos.y.max(end_pointer_pos.y) }) else {
                    return;
                };
                gizmos.linestrip(vec![left_bottom.get_point(RAY_RANGE),
                                      right_bottom.get_point(RAY_RANGE),
                                      right_top.get_point(RAY_RANGE),
                                      left_top.get_point(RAY_RANGE),
                                      left_bottom.get_point(RAY_RANGE)], Color::ANTIQUE_WHITE);
                if action_state.just_released(&PlayerMovement::Click) {
                    let intersections = spatial_query.shape_intersections(
                        &Collider::trimesh(vec![left_top.origin,
                                                right_top.origin,
                                                left_bottom.origin,
                                                right_bottom.origin,
                                                left_top.get_point(RAY_RANGE),
                                                right_top.get_point(RAY_RANGE),
                                                left_bottom.get_point(RAY_RANGE),
                                                right_bottom.get_point(RAY_RANGE)],
                                           vec![[0, 1, 2], // Side 0
                                                [2, 1, 3],
                                                [4, 0, 6], // Side 1
                                                [6, 0, 2],
                                                [7, 5, 6], // Side 2
                                                [6, 5, 4],
                                                [3, 1, 7], // Side 3
                                                [7, 1, 5],
                                                [4, 5, 0], // Side 4
                                                [0, 5, 1],
                                                [3, 7, 2], // Side 5
                                                [2, 7, 6]]),          // Shape
                        Vec3::ZERO,
                        Quat::default(),
                        SpatialQueryFilter::from_mask(GameLayer::RaycastInteractible),
                    );
                    let entity = intersections.get(0);
                    if let Some(entity_unwrapped) = entity {
                        let entity_unwrapped_clone = entity_unwrapped.clone();
                        let mut result =  q_door_egui_interactable_empty.get_mut(entity_unwrapped_clone);
                        if result.is_ok(){
                            if(q_door_egui_interactable_component.get_mut(entity_unwrapped_clone).is_err()){
                                commands.get_entity(entity_unwrapped_clone).unwrap().insert(result.unwrap().build_interactable_component());
                            }
                        }
                    }
                    pointer.start_click_pos = None;
                }
            } else {
                let Some(cursor) = camera.viewport_to_world(&transform, end_pointer_pos) else {
                    return;
                };
                let intersections = spatial_query.cast_ray(cursor.origin,
                                                           cursor.direction,
                                                           RAY_RANGE,
                                                           true,
                                                           SpatialQueryFilter::from_mask(GameLayer::RaycastInteractible),
                );
                if intersections.is_some() {
                    ctx.debug_painter().circle_filled(Pos2::from(pointer.current_pos.unwrap().as_ref()), 8.0, Color32::from_rgb(255, 255, 255));
                } else {
                    ctx.debug_painter().circle_filled(Pos2::from(pointer.current_pos.unwrap().as_ref()), 4.0, Color32::from_rgb(100, 100, 100));
                }
            }
        } else {
            ctx.debug_painter().circle_filled(Pos2::from(pointer.current_pos.unwrap().as_ref()), 4.0, Color32::from_rgb(100, 100, 100));

        }
    }
}
fn paused_update(mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
                 mut next_state: ResMut<NextState<MyAppState>>,
                 mut q_player: Query<&ActionState<PlayerMovement>, (With<CharacterController>,With<Camera>)>,
                 mut contexts: EguiContexts,
                 mut next_cursor_state: ResMut<NextState<AppCursorState>>,
                 mut settings: ResMut<Persistent<Settings>>,
                 mut q_pointer: Query<&mut VirtualPointer, With<VirtualPointer>>,
                 mut is_settings_open: Local<bool>) {
    let mut action_state = q_player.single_mut();
    let mut primary_window = q_windows.single_mut();
    if action_state.just_pressed(&PlayerMovement::Pause) {
        primary_window.cursor.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor.visible = false;
        next_cursor_state.set(AppCursorState::Locked);
        next_state.set(MyAppState::InGame);
    }
    let mut pointer = q_pointer.get_single_mut().unwrap();
    let ctx = contexts.ctx_mut();
    if let Some(current_pos) = primary_window.cursor_position() {
        ctx.debug_painter().circle_filled(Pos2::from(current_pos.as_ref()), 8.0, Color32::from_rgb(255, 255, 255));
    }

    if *(is_settings_open) {
        egui::Window::new("Settings").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Horizontal Look Sensitivity");
                if ui.add(egui::Slider::new(&mut settings.look_sensitivity, 1.0..=1000.0)).changed() {
                    settings.persist().expect("failed to save new settings");
                }
            });
            ui.horizontal(|ui| {
                ui.label("Cursor Horizontal Sensitivity");
                if ui.add(egui::Slider::new(&mut settings.mouse_sensitivity.x, 0.05..=2.0)).changed() {
                    settings.persist().expect("failed to save new settings");
                }
            });
            ui.horizontal(|ui| {
                ui.label("Cursor Vertical Sensitivity");
                if ui.add(egui::Slider::new(&mut settings.mouse_sensitivity.y, 0.05..=2.0)).changed() {
                    settings.persist().expect("failed to save new settings");
                }
            });
        });
    }
    egui::Window::new("Paused").show(ctx, |ui| {
        if ui.button("Settings").clicked() {
            *is_settings_open = !(*is_settings_open);
        }
    });
}

