use crate::action::Action;

pub type ActionId = u32;

#[derive(Debug)]
pub struct ActionWrapper {
    pub action: Box<dyn Action>,
    pub id: ActionId,
}
