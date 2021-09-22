# Introduction to Rocket

## Description

![Hero Image](rusty-rockets.png)

This repository contains a sample that I used to do an introduction talk about *Building Web APIs with Rust and Rocket* at the [Rust Linz](https://rust-linz.at) meetup.

# Storyboard

## Prepare Environment

```sh
cargo new rusty-rocket-live
code .
cp -R ../rusty-rocket/.vscode ./.vscode
```

## Dependencies

* Open *Cargo.toml*
* Snippet *010-rocket-dependency* in `[dependencies]`
* Snippet *015-rocket-git*
* `cargo build`

## Basics

* Open *main.rs*
* Snippet *020-use*, fold region
* Snippet *025-basic-get* and *030-basic-launch*

```sh
cp -R ../rusty-rocket/requests.http .
```

* Demo request

## Tests

* Create *src/tests.rs*
* Snippet *040-basic-test* in *tests.rs*
* Snippet *035-annotate-test-module* in *main.rs*
* `cargo test`

## Dynamic Paths

* Snippet *045-dynamic-path* in *main.rs*
* Add `greeting` to mounts
* Demo request
* Snippet *050-dynamic-path-test* in *test.rs*
* `cargo test`

## Querystring Parameters

* Snippet *055-query-string-params* in *main.rs*
* Add `query_greeting` to mounts
* Demo request
* Snippet *060-query-string-tests* in *test.rs*
* `cargo test`

## Request Guards

* Create *src/api_key.rs*
* Snippet *065-custom-request-guard* in *api-key.rs*
* Snippet *070-route-with-guard* in *main.rs*
* Add `protected` to mounts
* Demo request
* Snippet *075-tests-guarded-route* in *test.rs*
* `cargo test`

## Cookie Guards

* Snippet *080-cookie-guard* in *main.rs*
* Add `login` and `session` to mounts
* Demo request
* Snippet *085-cookie-test* in *test.rs*
* `cargo test`

## Simple REST API

* Snippet *090-hero-api-region*
* Inside:
  * Snippet *100-hero-api-structs-types*
  * Snippet *105-in-memory-repository*
  * Snippet *110-add-hero-with-post*
  * Snippet *111-add-managed-hashmap*
  * Snippet *115-get-single-hero*
  * Snippet *120-get-all-heroes*
* Add `add_hero`, `get_hero`, and `get_all` to mounts
* At the end: Snippet *111-add-managed-hashmap*
* Demo request

## Catcher

* Snippet *125-404-catcher*
* Snippet *126-register-catcher*
* Demo request

## Fairings

* Snippet *130-log-fairing*
* Snippet *135-attach-fairing*
* Execute some demo requests (GET and POST)
* Show counter with demo request
