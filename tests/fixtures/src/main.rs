fn main() {
    println!("starting app");
    let user = authenticate("admin", "secret");
    match user {
        Some(name) => println!("welcome {}", name),
        None => println!("access denied"),
    }
}

fn authenticate(username: &str, password: &str) -> Option<String> {
    if username == "admin" && password == "secret" {
        Some(username.to_string())
    } else {
        None
    }
}
