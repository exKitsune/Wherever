use std::io;

fn main() -> io::Result<()> {
    let client = reqwest::blocking::Client::new();
    let addr = std::env::args()
        .skip(1)
        .next()
        .unwrap_or("127.0.0.1:8998".into());
    let resp = client
        .post(
            format!("http://{}/open", addr)
                .parse::<reqwest::Url>()
                .unwrap(),
        )
        .body("https://www.youtube.com/watch?v=mZ0sJQC8qkE")
        .send()
        .unwrap();
    Ok(())
}
