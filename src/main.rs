use kvarn::prelude::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut extensions = Extensions::empty();

    extensions.add_cors(
        Cors::new()
            .allow(
                "/api*",
                CorsAllowList::new().add_origin("https://icelk.dev"),
            )
            .build(),
    );

    extensions.add_prime(prime!(request, _host, _addr {
        if request.uri().path() == "/" {
            // This maps the Option<HeaderValue> to Option<Result<&str, _>> which the
            // `.and_then(Result::ok)` makes Option<&str>, returning `Some` if the value is both `Ok` and `Some`.
            // Could also be written as
            // `.get("user-agent").and_then(|header| header.to_str().ok())`.
            if let Some(ua) = request.headers().get("user-agent").map(HeaderValue::to_str).and_then(Result::ok) {
                if ua.contains("curl") {
                    Some(Uri::from_static("/ip"))
                } else {
                    Some(Uri::from_static("/index.html"))
                }
            } else {
                None
            }
        } else {
            None
        }
    }), extensions::Id::new(16, "Redirect `/`"));

    extensions.add_prepare_single(
        "/ip".to_string(),
        prepare!(_request, _host, _path, addr {
            let ip = addr.ip().to_string();
            let response = Response::new(Bytes::copy_from_slice(ip.as_bytes()));
            FatResponse::no_cache(response)
        }),
    );
    extensions.add_prepare_single(
        "/index.html".to_string(),
        prepare!(_request, _host, _path, addr {
            let content = format!(
                "!> simple-head Your IP address\n\
                <h2>Your IP address is {}</h2>",
                addr.ip()
            );
            let response = Response::new(Bytes::copy_from_slice(content.as_bytes()));
            FatResponse::new(response, ServerCachePreference::None)
        }),
    );

    extensions.add_present_internal(
        "simple-head".to_string(),
        present!(present_data {
            let content = present_data.response().body();

            let start = r#"
<!DOCTYPE html>
<html>
<head>
    <title>"#;
            let middle = r#"</title>
</head>
<body>"#;
            let end = r#"
</body>
</html>
"#;
            let title = present_data.args().iter().fold(String::new(), |mut acc, arg| {
                acc.push_str(arg);
                acc.push(' ');
                acc
            });

            let bytes = build_bytes!(start.as_bytes(), title.as_bytes(), middle.as_bytes(), &content, end.as_bytes());
            *present_data.response_mut().body_mut() = bytes;
        }),
    );
    extensions.add_package(
        package!(response, _request, _host {
            response.headers_mut().insert("fun-header", HeaderValue::from_static("why not?"));
            replace_header_static(response.headers_mut(), "content-security-policy", "default-src 'self'; style-src 'unsafe-inline' 'self'");
        }),
        extensions::Id::new(-1024, "add headers"),
    );
    extensions.add_post(
        post!(_request, host, _response_pipe, body, addr {
            if let Ok(mut body) = str::from_utf8(&body) {
                body = body.get(0..512).unwrap_or(body);
                println!("Sent {:?} to {} from {}", body, addr, host.name);
            }
        }),
        extensions::Id::new(0, "Print sent data"),
    );

    // Let's see which extensions are attached:
    println!("Our extensions: {:#?}", extensions);

    println!("Notice all the CORS extensions. We added the CORS handler, which gives us all the extensions, with the right configuration.");

    let host = Host::non_secure("localhost", "non-existent", extensions, host::Options::default());
    let data = Data::builder(host).build();
    let port = PortDescriptor::non_secure(8080, data);
    let server = run(vec![port]).await;

    println!("Started server at http://localhost:8080/");
    println!("Try http://127.0.0.1:8080/ for the IPv4 version.");
    println!("Test going to the page in a browser and the curling it, you'll get different results.");
    println!("Shutting down in 10 seconds.");

    let sendable_server_handle = Arc::clone(&server);

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        println!("Starting graceful shutdown");
        sendable_server_handle.shutdown();
    });

    server.wait().await;

    println!("Graceful shutdown complete.");
}
