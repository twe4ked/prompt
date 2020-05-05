use crate::component::Component;

pub fn display(jobs: Option<String>) -> Option<Component> {
    match jobs {
        Some(jobs) => Some(Component::Computed(jobs)),
        None => None,
    }
}
