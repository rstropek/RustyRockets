// region use
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]
use reqwest;
use rocket::{Build, Data, Request, Rocket, State, catch, catchers, fairing::{self, Fairing, Info, Kind}, get, http::{Cookie, CookieJar, Method}, launch, post, response::{content::Html, status::Created}, routes, serde::json::Json, uri};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
// endregion

#[cfg(test)]
mod tests;

// region Basic GET Request
// We can return basic data types like numbers, strings, Option, Result
// because Rocket contains ready-made implementation of the `Responder` trait
// for them. For our own types, we could implement custom responders.
//    (see https://rocket.rs/v0.4/guide/responses/#implementations)
#[get("/")]
fn hello_world() -> &'static str {
    "Hello, world!"
}
// endregion

// region Dynamic paths
// You can use any type that implements the `FromParam` trait
//    (see also https://api.rocket.rs/v0.4/rocket/request/trait.FromParam.html)
// Use `RawStr` to get unsanitized, unvalidated, and undecoded raw string from HTTP message
//    (see also https://api.rocket.rs/v0.4/rocket/http/struct.RawStr.html)
// Rust considers parameter types during routing
//    (see also https://rocket.rs/v0.4/guide/requests/#dynamic-paths)
#[get("/<name>")]
fn greeting(name: String) -> String {
    format!("Hello {}", name)
}
// endregion

// region Query string params
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
// endregion

// region Request guard
mod api_key;

// Request guard
#[get("/protected")]
fn protected(key: api_key::ApiKey) -> String {
    format!("You are allowed to access this API because you presented key '{}'", key.0)
}
// endregion

// region Cookie guard
// use rocket::{get, launch, routes, http::{Cookie,CookieJar,}};
// Cookie request guard. There are also private (=encrypted) cookies.
//    (see https://rocket.rs/v0.4/guide/requests/#cookies)
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
// endregion

// region Simple REST API
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
    heroes_state: &State<HeroesMap>,
    hero_count: &State<HeroCount>,
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
    let location = uri!("/api", get_hero(hid));
    Created::new(location.to_string()).body(Json(new_hero))
}

// Note that we return `Option`. `None` would result in 404 (not found).
#[get("/heroes/<id>")]
fn get_hero(id: ID, heroes_state: &State<HeroesMap>) -> Option<Json<Hero>> {
    let heroes = heroes_state.read().unwrap();
    heroes.get(&id).map(|h| Json(h.clone()))
}

#[get("/heroes")]
fn get_all(heroes_state: &State<HeroesMap>) -> Json<Vec<Hero>> {
    let heroes = heroes_state.read().unwrap();
    Json(heroes.values().map(|v| v.clone()).collect())
}
// endregion

// region Catcher for 404
// Catcher for 404 errors
//    (see https://rocket.rs/v0.4/guide/requests/#error-catchers)
#[catch(404)]
fn not_found() -> Html<&'static str> {
    Html(r#"
        <h1>Not found</h1>
        <p>What are you looking for?</p>
    "#)
}
// endregion

// region Count Fairing
// Implement a fairing that counts all requests
//    (more about fairings at https://rocket.rs/v0.4/guide/fairings/#fairings)
#[derive(Default, Clone)]
struct Counter {
    get: Arc<AtomicUsize>,
    post: Arc<AtomicUsize>,
}

#[rocket::async_trait]
impl Fairing for Counter {
    fn info(&self) -> Info {
        Info {
            name: "GET/POST Counter",
            kind: Kind::Ignite | Kind::Request
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        #[get("/api/counts")]
        fn counts(counts: &State<Counter>) -> String {
            let get_count = counts.get.load(Ordering::Relaxed);
            let post_count = counts.post.load(Ordering::Relaxed);
            format!("Get: {}\nPost: {}", get_count, post_count)
        }

        Ok(rocket.manage(self.clone()).mount("/", routes![counts]))
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        if request.method() == Method::Get {
            self.get.fetch_add(1, Ordering::Relaxed);
        } else if request.method() == Method::Post {
            self.post.fetch_add(1, Ordering::Relaxed);
        }
    }
}
// endregion

// Will generate (async) main function for us
#[launch]
fn rocket() -> _ {
    rocket::build()
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
                get_all,
            ],
        )
        // Add managed state. Here we use global state. Request-local state
        // would also be possible.
        //    (see https://rocket.rs/v0.4/guide/state/)
        .manage(RwLock::new(HashMap::<ID, Hero>::new()))
        .manage(HeroCount(AtomicUsize::new(1)))
        // Register catchers for errors.
        //    (see https://rocket.rs/v0.4/guide/requests/#error-catchers)
        .register("/", catchers![not_found])
        .attach(Counter::default())
}
