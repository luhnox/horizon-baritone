// What Baritone needs from the agent module: aim at a block, hold the attack button, click.
use super::Game;
use winit::event::MouseButton;

impl Game<'_> {
    pub(in crate::game) fn bar_aim(&mut self, pos: [i32; 3]) -> bool {
        self.aim_block(pos, None).is_ok()
    }
    /// Why a block cannot be aimed at from here, if it cannot.
    pub(in crate::game) fn bar_aim_why(&mut self, pos: [i32; 3]) -> Result<(), String> {
        self.aim_block(pos, None).map(|_| ()).map_err(|e| e.to_string())
    }
    pub(in crate::game) fn bar_attack(&mut self, down: bool) {
        let _ = self.agent_mouse(MouseButton::Left, down);
    }
    pub(in crate::game) fn bar_attacking(&self) -> bool {
        self.left
    }
    /// Look at `face` of `against` and click: puts the held block against it, or the held bucket's
    /// water, or scoops it.
    pub(in crate::game) fn bar_place(&mut self, against: [i32; 3], face: [i32; 3]) -> bool {
        if self.aim_block(against, Some(face)).is_err() {
            return false;
        }
        let _ = self.agent_mouse(MouseButton::Right, true);
        let _ = self.agent_mouse(MouseButton::Right, false);
        true
    }
    /// Why a block cannot be clicked from here, if it cannot.
    pub(in crate::game) fn bar_place_why(&mut self, against: [i32; 3], face: [i32; 3]) -> Result<(), String> {
        self.aim_block(against, Some(face)).map(|_| ()).map_err(|e| e.to_string())
    }
    /// Look at a block and use it: a door opens.
    pub(in crate::game) fn bar_interact(&mut self, pos: [i32; 3]) -> bool {
        if self.aim_block(pos, None).is_err() {
            return false;
        }
        let _ = self.agent_mouse(MouseButton::Right, true);
        let _ = self.agent_mouse(MouseButton::Right, false);
        true
    }
    /// Use what is in the hand: a firework rocket while gliding.
    pub(in crate::game) fn bar_use(&mut self) {
        let _ = self.agent_mouse(MouseButton::Right, true);
        let _ = self.agent_mouse(MouseButton::Right, false);
    }
    /// The block under the crosshair, within 64 blocks.
    pub(in crate::game) fn bar_look_target(&self) -> Option<[i32; 3]> {
        let eye = self.eye_position()?;
        let dir = super::super::view_vector(self.yaw, self.pitch);
        self.clip_ray(eye, dir, 64.0).map(|hit| hit.pos)
    }
    /// GuiClick.render: the block under the mouse, as far as the view goes (the far plane the Java unprojects to).
    pub(in crate::game) fn bar_mouse_block(&mut self) -> Option<[i32; 3]> {
        let partial = self.clock.camera_fraction() as f32;
        let (camera, yaw, pitch) = self.camera_view(partial)?;
        let origin = camera.to_array();
        let fov = self.fov_degrees(partial, origin);
        let dir = super::super::baritone_gui_click::ray_through(self.cursor, self.gui_size(), fov, yaw, pitch);
        let reach = f64::from(self.settings.render_distance) * 16.0 * 4.0;
        self.clip_ray(origin, dir, reach).map(|hit| hit.pos)
    }
    /// The player's eyes, and how far a hand reaches.
    pub(in crate::game) fn bar_eye(&self) -> Option<[f64; 3]> {
        self.eye_position()
    }
    pub(in crate::game) fn bar_reach(&self) -> f64 {
        self.reach()
    }
    /// The block an aim hits from `eye` within `reach`, and the face hit (RayTraceUtils.rayTraceTowards).
    pub(in crate::game) fn bar_ray(&self, eye: [f64; 3], yaw: f32, pitch: f32, reach: f64) -> Option<([i32; 3], [i32; 3])> {
        let dir = super::super::view_vector(yaw, pitch);
        self.clip_ray(eye, dir, reach).map(|hit| (hit.pos, hit.face))
    }
}
