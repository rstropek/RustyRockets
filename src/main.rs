use reqwest;
use rocket::{
    catch, catchers,
    fairing::{Fairing, Info, Kind},
    get,
    http::{Cookie, CookieJar,},
    launch, post,
    response::status::Created,
    routes, uri, Data, Request, State,
};
use rocket_contrib::{
    json,
    json::{Json, JsonValue},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;

mod api_key;
#[cfg(test)]
mod tests;

// We can return basic data types like numbers, strings, Option, Result
// because Rocket contains ready-made implementation of the `Responder` trait
// for them. For our own types, we could implement custom responders.
//    (see https://rocket.rs/v0.4/guide/responses/#implementations)
#[get("/")]
fn hello_world() -> &'static str {
    "Hello, world!"
}

// Dynamic paths
// You can use any type that implements the `FromParam` trait
//    (see also https://api.rocket.rs/v0.4/rocket/request/trait.FromParam.html)
// Use `RawStr` to get unsanitized, unvalidated, and undecoded raw string from HTTP message
//    (see also https://api.rocket.rs/v0.4/rocket/http/struct.RawStr.html)
#[get("/<name>")]
fn greeting(name: String) -> String {
    format!("Hello {}", name)
}

// Query string params
// You can use any type that implements the `FromParam` trait
//    (see also https://api.rocket.rs/v0.4/rocket/request/trait.FromFormValue.html)
// Note optional parameters
// For more details see https://rocket.rs/v0.4/guide/requests/#query-strings
#[get("/hello?<name>&<salutation>")]
fn query_greeting(name: String, salutation: Option<String>) -> String {
    match salutation {
        Some(s) => format!("{} {}", s, name),
        None => format!("Hello {}", name),
    }
}

// Request guard
#[get("/protected")]
fn protected(key: api_key::ApiKey) -> String {
    format!(
        "You are allowed to access this API because you presented key '{}'",
        key.0
    )
}

// Cookie request guard
#[get("/login")]
fn login(cookies: &CookieJar) {
    cookies.add(Cookie::new(
        "Session",
        base64::encode("this_is_a_session_key"),
    ));
}

#[get("/session")]
fn session(cookies: &CookieJar) -> &'static str {
    match cookies.get("Session") {
        Some(_) => "You got the cookie!",
        None => "Sorry, no cookie!",
    }
}

type ID = usize;

// Rocket uses Serde for serializing/deserializing data.
#[derive(Serialize, Debug, Clone)]
struct Hero {
    id: ID,
    name: String,
    #[serde(rename(serialize = "canFly"))]
    can_fly: bool,
}

#[derive(Deserialize, Debug)]
struct NewHero {
    name: String,
    #[serde(rename(deserialize = "canFly"))]
    can_fly: bool,
}

// We use a `RwLock`-protected `HashMap` instead of a DB. Note that Rocket has
// built-in support for databases, but this is out-of-scope of this demo.
//    (see https://rocket.rs/v0.4/guide/state/#databases for DB support)
struct HeroCount(AtomicUsize);
type HeroesMap = RwLock<HashMap<ID, Hero>>;

// Rocket processes body data based on argument types. Here we deserialize
// (`Deserialize` trait from Serde) a `NewHero` into the `hero` argument.
//    (see https://rocket.rs/v0.4/guide/requests/#json)
// Note that we return `Created`. It is a wrapping responder that changes the
// HTTP status code to 201 (created) and responds with the inner responder
// (in this case JSON).
//    (see https://rocket.rs/v0.4/guide/responses/#wrapping)
#[post("/heroes", format = "json", data = "<hero>")]
fn add_hero(
    hero: Json<NewHero>,
    heroes_state: State<'_, HeroesMap>,
    hero_count: State<'_, HeroCount>,
) -> Created<Json<Hero>> {
    // Generate unique hero ID
    let hid = hero_count.0.fetch_add(1, Ordering::Relaxed);

    // Build new hero
    let new_hero = Hero {
        id: hid,
        name: hero.0.name,
        can_fly: hero.0.can_fly,
    };

    // Insert new hero in hashmap
    let mut heroes = heroes_state.write().unwrap();
    heroes.insert(hid, new_hero.clone());

    // Use uri macro to generate location header
    //    (see https://rocket.rs/v0.4/guide/responses/#typed-uris)
    let location = uri!("/api", get_hero: hid);
    Created::new(location.to_string()).body(Json(new_hero))
}

// Note that we return `Option`. `None` would result in 404 (not found).
#[get("/heroes/<id>")]
fn get_hero(id: ID, heroes_state: State<'_, HeroesMap>) -> Option<Json<Hero>> {
    let heroes = heroes_state.read().unwrap();
    heroes.get(&id).map(|h| Json(h.clone()))
}

#[get("/heroes")]
fn get_all(heroes_state: State<'_, HeroesMap>) -> Json<Vec<Hero>> {
    let heroes = heroes_state.read().unwrap();
    Json(heroes.values().map(|v| v.clone()).collect())
}

// Catcher for 404 errors
//    (see https://rocket.rs/v0.4/guide/requests/#error-catchers)
#[catch(404)]
fn not_found() -> JsonValue {
    json!({
        "status": "error",
        "reason": "Resource was not found."
    })
}

// Add a structure for event log entries
#[derive(Serialize, Debug)]
struct LogEvent {
    #[serde(rename(serialize = "@t"))]
    timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(rename(serialize = "@mt"))]
    message_template: &'static str,
    #[serde(rename(serialize = "path"))]
    path: String,
}

// Implement a fairing that sends a log entry for each incoming request
// to a central logging system (here: Seq)
//    (more about fairings at https://rocket.rs/v0.4/guide/fairings/#fairings)
struct LogTarget;

#[rocket::async_trait]
impl Fairing for LogTarget {
    fn info(&self) -> Info {
        Info {
            name: "Log to Seq",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data) {
        // Implement a rather naive middleware that sends a log entry
        // to Seq on every request. In practice, you would probably batch
        // sending of log entries.
        let event = LogEvent {
            timestamp: chrono::Utc::now(),
            message_template: "Request to {path}",
            path: request.uri().path().to_owned(),
        };
        let client = reqwest::Client::new();
        client
            .post("http://192.168.1.7:5341/api/events/raw?clef")
            .json(&event)
            .send()
            .await
            .unwrap();
    }
}

// Will generate (async) main function for us
#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount(
            "/api",
            routes![
                hello_world,
                greeting,
                query_greeting,
                protected,
                login,
                session,
                add_hero,
                get_hero,
                get_all
            ],
        )
        // Add managed state. Here we use global state. Request-local state
        // would also be possible.
        //    (see https://rocket.rs/v0.4/guide/state/)
        .manage(RwLock::new(HashMap::<ID, Hero>::new()))
        .manage(HeroCount(AtomicUsize::new(1)))
        // Register catchers for errors.
        //    (see https://rocket.rs/v0.4/guide/requests/#error-catchers)
        .register(catchers![not_found])
        .attach(LogTarget {})
}
