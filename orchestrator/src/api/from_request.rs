//! Implementations in this file are useful to allow accessing
//! robot state and queue handler directly from Rocket routes:
//! ```rust,no_run
//! #[get("/hello")]
//! fn hello(robot_state: State, queue_handler: &QueueHandler) -> String {
//!     format!("Hello, state={:?} queue={:?}", robot_state, queue_handler)
//! }
//! ```

use rocket::{request::{self, FromRequest}, Request};

use crate::{queue::QueueHandler, state::{State, StateHandler}};


#[rocket::async_trait]
impl<'r> FromRequest<'r> for State {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request.guard::<&rocket::State<StateHandler>>().await
            .map(|request_handler| request_handler.get_state())
    }
}

// why doesn't Rocket provide directly &QueueHandler,
// but only &State<QueueHandler>, thus requiring this FromRequest impl?
#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r QueueHandler {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request.guard::<&rocket::State<QueueHandler>>().await
            .map(|request_handler| request_handler.inner())
    }
}