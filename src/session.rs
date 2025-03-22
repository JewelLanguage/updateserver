use std::collections::HashMap;
use crate::{Action, Platform, Request, Response};
use crate::Action::latest;

#[derive(Clone)]
pub struct Session {
    pub requestid: String,
    pub possible_actions: Vec<Action>,
    pub previous_action: Action,
}

pub struct Session_Manager {
    pub sessions: HashMap<String, Session>,
}

pub fn new_session_manager() -> Session_Manager {
    Session_Manager {
        sessions: HashMap::new(),
    }
}

pub fn new_session(manager: &mut Session_Manager, request: &Request) -> bool {
    if manager.sessions.contains_key(&request.sessionid) {
        return false;
    }
    manager.sessions.insert(
        request.sessionid.clone(),
        Session {
            requestid: request.requestid.clone(),
            possible_actions: vec![Action::latest],
            previous_action: Action::latest,
        },
    );
    true
}

pub fn update_session_actions(
    manager: &mut Session_Manager,
    request: &Request,
    new_actions: Vec<Action>,
) -> (bool, String) {
    if let Some(session) = manager.sessions.get_mut(&request.sessionid) {
        session.possible_actions = new_actions;
        return (true, String::from("success"));
    }
    (false, String::from("Invalid Session ID"))
}

pub fn update_current_action(
    manager: &mut Session_Manager,
    request: &Request,
    new_action: Action,
) -> (bool, String) {
    if let Some(session) = manager.sessions.get_mut(&request.sessionid) {
        session.previous_action = new_action;
        return (true, String::from("success"));
    }
    (false, String::from("Invalid Session ID"))
}

pub fn update_request(
    manager: &mut Session_Manager,
    request: &Request,
    new_requestid: String,
) -> (bool, String) {
    if let Some(session) = manager.sessions.get_mut(&request.sessionid) {
        session.requestid = new_requestid.clone();
        return (true, String::from("success"));
    }
    (false, String::from("Invalid Session ID"))
}

pub fn remove_session(manager: &mut Session_Manager, sessionid: String) -> bool {
    manager.sessions.remove(&sessionid).is_some()
}
