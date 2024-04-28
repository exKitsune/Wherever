use std::io;

fn main() -> io::Result<()> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("http://127.0.0.1:8998/open")
        .body("https://www.youtube.com/watch?v=mZ0sJQC8qkE")
        .send()
        .unwrap();
    Ok(())
}
