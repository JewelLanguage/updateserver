use std::collections::HashMap;
use crate::{Action, Platform, Request, Response};
use crate::Action::latest;

pub struct Session{
    pub requestid:String,
    pub possible_actions:Vec<Action>,
    pub current_action:Action
}

pub struct Session_Manager{
    sessions: HashMap<String, Session>
}

pub fn new_session_manager() -> Session_Manager{
    Session_Manager{
        sessions: HashMap::new()
    }
}

pub fn new_session(mut manager: &mut Session_Manager, request:&Request) -> bool{
    if(!&manager.sessions.contains_key(&request.sessionid)){
       return false
    }

    manager.sessions.insert(request.sessionid.clone(), Session {requestid:request.requestid.clone(), possible_actions:vec![Action::latest], current_action:Action::latest});
    true
}

pub fn update_session_actions(mut manager:&Session_Manager, request:&Request, new_actions:Vec<Action>) ->(bool, String) {

    if(!&manager.sessions.contains_key(&request.sessionid)){
        return (false, String::from("Invalid Session ID"));
    }

    if(&manager.sessions[&request.sessionid].requestid != &request.requestid){
        return (false, String::from("Invalid Request ID"));
    }

    manager.sessions[&request.sessionid].possible_actions = new_actions;

    (true, String::from("success"))

}

pub fn update_current_action(mut manager:&Session_Manager, request: &Request, new_action:Action) -> (bool, String) {
    if(!&manager.sessions.contains_key(&request.sessionid)){
        return (false, String::from("Invalid Session ID"));
    }

    if(&manager.sessions[&request.sessionid].requestid != &request.requestid){
        return (false, String::from("Invalid Request ID"));
    }

    manager.sessions[&request.sessionid].current_action = new_action;
    (true, String::from("success"))
}

pub fn update_request(mut manager:&Session_Manager, request: &Request, new_requestid:String) -> (bool, String) {
    if(!&manager.sessions.contains_key(&request.sessionid)){
        return (false, String::from("Invalid Session ID"));
    }

    if(&manager.sessions[&request.sessionid].requestid != &request.requestid){
        return (false, String::from("Invalid Request ID"));
    }

    manager.sessions[&request.sessionid].requestid = new_requestid;
    (true, String::from("success"))
}

pub fn remove_session(mut manager:Session_Manager, sessionid:String) -> bool {
    if(!&manager.sessions.contains_key(&sessionid)){
        return false;
    }

    manager.sessions.remove(&sessionid);
    true
}