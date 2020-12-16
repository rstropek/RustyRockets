use rocket::{get, post, launch, routes, http::{Cookie,CookieJar,}, State, response::{status::Created,}, uri, catch, catchers};
use std::{str,};
use serde::{Serialize, Deserialize};
use rocket_contrib::{json::{Json, JsonValue}, json};
use std::sync::{RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)] mod tests;
mod api_key;

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
        None => format!("Hello {}", name)
    }
}

// Request guard
#[get("/protected")]
fn protected(key: api_key::ApiKey) -> String {
    format!("You are allowed to access this API because you presented key '{}'", key.0)
}

// Cookie request guard
#[get("/login")]
fn login(cookies: &CookieJar) {
    cookies.add(Cookie::new("Session", base64::encode("this_is_a_session_key")));
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
    can_fly: bool
}

#[derive(Deserialize, Debug)]
struct NewHero {
    name: String,
    #[serde(rename(deserialize = "canFly"))]
    can_fly: bool
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
fn add_hero(hero: Json<NewHero>, heroes_state: State<'_, HeroesMap>, hero_count: State<'_, HeroCount>) -> Created<Json<Hero>> {
    // Generate unique hero ID
    let hid = hero_count.0.fetch_add(1, Ordering::Relaxed);

    // Build new hero
    let new_hero = Hero{ id: hid, name: hero.0.name, can_fly: hero.0.can_fly };

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
    heroes.get(&id).map(|h| { Json(h.clone())})
}

#[get("/heroes")]
fn get_all(heroes_state: State<'_, HeroesMap>) -> Json<Vec<Hero>> {
    let heroes = heroes_state.read().unwrap();
    Json(heroes.values().map(|v| { v.clone()}).collect())
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

// Will generate (async) main function for us
#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/api", routes![hello_world, greeting, query_greeting, protected, login, session, add_hero, get_hero, get_all])
        // Add managed state. Here we use global state. Request-local state
        // would also be possible.
        //    (see https://rocket.rs/v0.4/guide/state/)
        .manage(RwLock::new(HashMap::<ID, Hero>::new()))
        .manage(HeroCount(AtomicUsize::new(1)))
        // Register catchers for errors.
        //    (see https://rocket.rs/v0.4/guide/requests/#error-catchers)
        .register(catchers![not_found])
}
