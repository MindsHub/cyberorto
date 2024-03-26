use crate::action::Action;

pub type ActionId = u32;

#[derive(Debug)]
pub struct ActionWrapper {
    pub action: Option<Box<dyn Action>>, // will be None if this is a placeholder
    pub id: ActionId,
}

impl ActionWrapper {
    pub fn make_placeholder_and_extract(&mut self) -> ActionWrapper {
        assert!(self.action.is_some()); // can't make a placeholder... a placeholder
        let mut new_action_wrapper = ActionWrapper {
            action: None,
            id: self.id,
        };
        std::mem::swap(self, &mut new_action_wrapper);
        new_action_wrapper
    }
}