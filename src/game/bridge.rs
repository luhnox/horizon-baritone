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
}
