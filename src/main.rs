#[macro_use] extern crate iron;
extern crate iron_sessionstorage;
extern crate router;
extern crate staticfile;
extern crate mount;
extern crate urlencoded;
#[macro_use] extern crate diesel;
extern crate dotenv;
#[macro_use] extern crate diesel_codegen;
extern crate rustc_serialize;
extern crate regex;
extern crate bcrypt;

use std::path::Path;
use std::fs::File;
use std::io::Read;
use iron::prelude::*;
use iron::status;
use iron::headers::ContentType;
use iron::modifiers::Header;
use router::Router;
use staticfile::Static;
use mount::Mount;
use iron_sessionstorage::SessionStorage;
use iron_sessionstorage::backends::SignedCookieBackend;
use urlencoded::UrlEncodedBody;
use regex::Regex;

pub mod schema;
pub mod models;
pub mod users;
pub mod session;
pub mod utils;

use utils::*;

fn index(req: &mut Request) -> IronResult<Response> {
    let mut file = if try!(session::is_logged_in(req)) {
        File::open("public/session_index.html").unwrap()
    } else {
        File::open("public/index.html").unwrap()
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    Ok(Response::with((status::Ok, Header(ContentType::html()), contents)))
}

fn get_session(req: &mut Request) -> IronResult<Response> {
    let session = if try!(session::is_logged_in(req)) {
        session::get_username(req)
            .unwrap()
            .unwrap()
            .to_string()
    } else {
        String::from("No session")
    };

    Ok(Response::with((status::Ok, session)))
}

fn set_session(req: &mut Request) -> IronResult<Response> {
    try!(session::set_username(req, String::from("eh lemme")));
    Ok(Response::with((status::Ok, "set")))
}

fn delete_session(req: &mut Request) -> IronResult<Response> {
    try!(session::delete_username(req));
    Ok(Response::with((status::Ok, "Deleted")))
}

fn register(req: &mut Request) -> IronResult<Response> {
    let username;
    let email;
    let name;
    let password;
    {
        let form = iexpect!(req.get_ref::<UrlEncodedBody>().ok(), error("form", "Please provide form data!"));
        username = iexpect!(form_field(form, "username"), error("username", "Username is required"));
        email = iexpect!(form_field(form, "email"), error("email", "Email is required"));
        name = iexpect!(form_field(form, "name"), error("name", "Name is required"));
        password = iexpect!(form_field(form, "password"), error("password", "Password is required"));
    }

    let conn = establish_connection();
    let old_user = users::get_by_username(&conn, &username);
    if old_user.is_some() {
        return Ok(Response::with(error("username", "Username taken")));
    }

    let email_regex = Regex::new(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}$").unwrap();
    if !email_regex.is_match(&email[..]) {
        return Ok(Response::with(error("email", "Invalid email")));
    }

    if password.len() < 5 {
        return Ok(Response::with(error("password", "Password too short!")));
    }

    let _user = users::create_user(&conn, &username, email, name, password);
    session::set_username(req, username).unwrap();

    Ok(Response::with(success()))
}

fn logout(req: &mut Request) -> IronResult<Response> {
    try!(session::delete_username(req));
    Ok(Response::with((status::Ok, success())))
}

fn main() {
    let mut api_router = Router::new();
    api_router.post("/register", register, "register");
    api_router.post("/logout", logout, "logout");
    // TODO: Use for login API
    api_router.get("/get_session", get_session, "get_session");
    api_router.get("/set_session", set_session, "set_session");
    api_router.get("/delete_session", delete_session, "delete_session");

    let session_secret = b"verysecret".to_vec();

    let mut mount = Mount::new();
    mount
        .mount("/", index)
        .mount("/api/", api_router)
        .mount("/public/", Static::new(Path::new("public/")));

    let mut ch = Chain::new(mount);
    ch.link_around(SessionStorage::new(SignedCookieBackend::new(session_secret)));
    let _res = Iron::new(ch).http("localhost:3000");
}
