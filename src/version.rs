use std::{
    io::{prelude::*, BufReader},
    fmt
};
use std::fmt::Debug;
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Version{
    pub major:i32,
    pub minor:i32,
    pub build:i32,
    pub patch:i32,
    pub count:i32,   // Number of successful downloads of this version
    pub urls:Vec<String>,
}

impl fmt::Display for Version{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}, {:?}", self.major, self.minor, self.build, self.urls)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Versions{
    pub dev:Vec<Version>,
    stable:Vec<Version>,
    beta:Vec<Version>,
    canary:Vec<Version>,
    extended:Vec<Version>
}

impl fmt::Display for Versions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Versions:\n")?;
        write!(f, "  Dev:    {:?}\n", self.dev)?;
        write!(f, "  Stable: {:?}\n", self.stable)?;
        write!(f, "  Beta:   {:?}\n", self.beta)?;
        write!(f, "  Canary: {:?}\n", self.canary)?;
        write!(f, "  Extended: {:?}\n", self.extended)?;
        Ok(())
    }
}
