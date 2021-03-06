#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate log;
#[macro_use] extern crate rocket;
extern crate env_logger;
extern crate rust_captcha;
extern crate serde_json;

use std::env;

use rust_captcha::requesthandler::{req_captcha_new, req_captcha_newget, req_captcha_solution};
use rust_captcha::methods::CaptchaError;
use rocket::response::content;
use serde_json::{json, Value};
use rocket::request::FromRequest;
use rocket::{Request, request};

const PORT: u16 = 8000;

fn precondition_checks() -> bool {
    match env::var("REDIS_HOST") {
        Err(_) => {
            error!("Environment variable REDIS_HOST not set.");
            false
        },
        Ok(_)  => true
    }
}

struct ClientId(String);

#[derive(Debug)]
struct ClientIdError;

fn client_id(cid: ClientId) -> String { // TODO validate
    match cid {
        ClientId(val) => val
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for ClientId {
    type Error = ClientIdError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let client_ids: Vec<_> = request.headers().get("x-client-id").collect();
        match client_ids.len() {
            0 => request::Outcome::Success(ClientId(String::from("<unknown>"))),
            _ => request::Outcome::Success(ClientId(client_ids[0].to_string()))
        }
    }
}

#[derive(Clone)]
enum CResult {
    Processed = 0,
    InternalError = 1,
    InvalidParameters = 2
}

fn error(code: CResult) -> Value {
    let result_str = vec!["processed", "internal error", "invalid parameters"];
    json!({
        "error_code": code.clone() as u32,
        "error_msg": result_str[code as usize],
        "result": ""
    })
}

fn not_found(code: CResult) -> Value {
    let result_str = vec!["processed", "internal error", "invalid parameters"];
    json!({
        "error_code": code.clone() as u32,
        "error_msg": result_str[code as usize],
        "result": json!({
            "solution": "not found",
            "trials_left": 0
        })
    })
}

fn create_response(r: Result<String, CaptchaError>) -> content::Json<String> {
    let result_str = vec!["processed", "internal error", "invalid parameters"];
    let ret = match r {
        Err(e) => {
            match e {
                CaptchaError::InvalidParameters => error(CResult::InvalidParameters),
                CaptchaError::CaptchaGeneration => error(CResult::InternalError),
                CaptchaError::Uuid => error(CResult::InternalError),
                CaptchaError::ToJson => error(CResult::InternalError),
                CaptchaError::Persist => error(CResult::InternalError),
                CaptchaError::NotFound => not_found(CResult::Processed),
                CaptchaError::Unexpected => error(CResult::InternalError)
            }
        },
        Ok(json) => {
            let data: Value = serde_json::from_str(&json).unwrap();
            json!({
                "error_code": CResult::Processed as u32,
                "error_msg": result_str[CResult::Processed as usize],
                "result": data
            })
        }
    };

    content::Json(ret.to_string())
}

#[post("/new/<difficulty>/<max_tries>/<ttl>")]
fn new(difficulty: String, max_tries: String, ttl: String, clientid: ClientId) -> content::Json<String> {
    create_response(req_captcha_new(difficulty, max_tries, ttl, client_id(clientid)))
}

#[get("/new/<difficulty>")]
fn new_diff_only(difficulty: String, clientid: ClientId) -> content::Json<String> {
    create_response(req_captcha_newget(difficulty, client_id(clientid)))
}

#[post("/solution/<id>/<solution>")]
fn solution(id: String, solution: String, clientid: ClientId) -> content::Json<String> {
    create_response(req_captcha_solution(id, solution, client_id(clientid)))
}


fn main() {
    env_logger::init();

    if !precondition_checks() {
        error!("Failed to start server.");
        return;
    }

    info!("Starting service on port {} ...", PORT);
    rocket::ignite()
        .mount("/", routes![new, new_diff_only, solution])
        .launch();
}
