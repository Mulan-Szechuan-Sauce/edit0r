use crate::RenderContext;

pub trait MajorMode {
    fn draw(&mut self, context: &mut RenderContext) -> Result<(), String>;
}
