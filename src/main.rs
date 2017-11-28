extern crate hyper;
extern crate iron;
extern crate params;
extern crate router;
extern crate typemap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate mime;

use hyper::header::ContentType;
use hyper::status::StatusCode;
use iron::typemap::Key;
use iron::prelude::*;
use iron::{Chain, BeforeMiddleware};
use router::Router;
use std::fmt;
use std::error::Error;
use std::collections::BTreeMap;
use params::Params;

lazy_static! {
    pub static ref TEXT_PLAIN: ContentType = ContentType(mime!(Text/Plain; Charset=Utf8));
}

#[derive(Debug)]
struct Fail(String);

impl fmt::Display for Fail {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.description().fmt(f)
    }
}

impl Error for Fail {
    fn description(&self) -> &str {
        self.0.as_str()
    }
}


#[derive(Debug)]
struct Params1 {
    first: String,
    last: String,
}

#[derive(Debug)]
struct Params2 {
    first: String,
    middle: String,
    last: String,
}

struct Key1;

impl Key for Key1 {
    type Value = Params1;
}

fn get(m: Option<&String>, name: &str) -> IronResult<String> {
    m.ok_or(IronError::new(Fail(String::from(format!("{} was absent", name))),
                              StatusCode::BadRequest))
        .map(|s| s.to_owned())
}

fn get_map(req: &mut Request) -> IronResult<BTreeMap<String, String>> {
    req.get_ref::<Params>()
        .or(Err(IronError::new(Fail(String::from("Parameter parse failed")),
                               StatusCode::BadRequest)))?
        .to_strict_map::<String>()
        .ok_or(IronError::new(Fail(String::from("Parameter parse failed")),
                              StatusCode::BadRequest))
}

impl BeforeMiddleware for Key1 {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let m = get_map(req)?;
        println!("Key1: {:?}", m);
        let pp = Params1 {
            first: get(m.get("first"), "first")?,
            last: get(m.get("last"), "last")?,
        };
        req.extensions.insert::<Key1>(pp);
        Ok(())
    }
}

struct Key2;

impl Key for Key2 {
    type Value = Params2;
}

impl BeforeMiddleware for Key2 {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        let m = get_map(req)?;
        println!("Key2: {:?}", m);
        let pp = Params2 {
            first: get(m.get("first"), "first")?,
            middle: get(m.get("middle"), "middle")?,
            last: get(m.get("last"), "last")?,
        };
        req.extensions.insert::<Key2>(pp);
        Ok(())
    }
}

trait Proc<P> {
    fn run(p: &P) -> IronResult<Response>;
}

impl Proc<Params1> for Params1 {
    fn run(p: &Params1) -> IronResult<Response> {
        Ok(Response::with((TEXT_PLAIN.0.clone(),
                           StatusCode::Ok,
                           format!("Hello {}, {}!", p.first, p.last))))
    }
}

impl Proc<Params2> for Params2 {
    fn run(p: &Params2) -> IronResult<Response> {
        Ok(Response::with((TEXT_PLAIN.0.clone(),
                           StatusCode::Ok,
                           format!("Hello {}, {}, {}!", p.first, p.middle, p.last))))
    }
}


fn hello<K: Key>(req: &mut Request) -> IronResult<Response>
    where K::Value: Proc<K::Value>
{
    println!("hello");
    let p = req.extensions.get::<K>().unwrap();
    K::Value::run(p)
}

macro_rules! chain {
    ($k:ident, $h:ident) => {
        {
            let mut c = Chain::new($h::<$k>);
            c.link_before($k);
            c
        }
    };
}

pub fn main() {
    let mut router = Router::new();
    router.get("/hello1", chain!(Key1, hello), "hello1");
    router.get("/hello2", chain!(Key2, hello), "hello2");

    Iron::new(router).http("0.0.0.0:3000").expect("Unable to start server");
}
