use std::{collections::HashMap, net::Ipv6Addr};

use serde::Serialize;
use wasm_bindgen::JsValue;
use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_log!("incoming ddns request");
    let req_params = get_request_params(req)?;
    if is_valid_secret(&req_params, &env)? {
        let ip6 = build_aaaa_record_value(&req_params, &env)?;
        update_aaaa_record(ip6, &env).await?;
        return Response::ok("DNS record updated");
    }
    Response::error("Could't process ddns request", 500)
}

fn get_request_params(req: Request) -> Result<HashMap<String, String>> {
    console_log!("loading request parameters");
    let url = req.url()?;
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    Ok(params)
}

fn is_valid_secret(req_params: &HashMap<String, String>, env: &Env) -> Result<bool> {
    let req_secret = env.secret("DDNS_SECRET")?.to_string();
    if let Some(secret) = req_params.get("secret") {
        if secret.clone() == req_secret {
            console_log!("secret of request is valid");
            return Ok(true);
        }
    }
    console_warn!("request with invalid secret received");
    Ok(false)
}

fn build_aaaa_record_value(req_params: &HashMap<String, String>, env: &Env) -> Result<Ipv6Addr>{
    let ip6prefix = req_params.get("ip6prefix").unwrap().split("/").collect::<Vec<&str>>()[0].parse::<Ipv6Addr>().unwrap().segments();
    let ip6token = env.var("DDNS_IP6_TOKEN")?.to_string().parse::<Ipv6Addr>().unwrap().segments();
    let ip6 = Ipv6Addr::new(
        ip6prefix[0],
        ip6prefix[1],
        ip6prefix[2],
        ip6prefix[3],
        ip6token[4],
        ip6token[5],
        ip6token[6],
        ip6token[7],
    );
    Ok(ip6)
}

async fn update_aaaa_record(ip6: Ipv6Addr, env: &Env) -> Result<()> {
    let zone_id = env.var("CLOUDFLARE_ZONE_ID")?.to_string();
    let dns_record_id = env.var("CLOUDLFARE_RECORD_ID")?.to_string();
    let req_url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}", zone_id, dns_record_id);
     
    let api_key = env.secret("CLOUDFLARE_API_KEY")?.to_string();
    let mut headers = Headers::new();
    headers.set("Authorization", format!("Bearer {}", api_key).as_str())?;
    headers.set("Content-Type", "application/json")?;

    let dns_update_request = DnsUpdateRequest {
        r#type: "AAAA".to_string(),
        name: "@".to_string(),
        content: ip6.to_string(),
        ttl: 1,
        proxied: true,
    };
    let body = Some(JsValue::from_str(&serde_json::to_string(&dns_update_request)?));

    let req_init = RequestInit {
        method: Method::Patch,
        headers,
        body,
        ..Default::default()
    };
    let request = Request::new_with_init(&req_url, &req_init)?;
    let response = Fetch::Request(request).send().await?.text().await?;
    console_log!("dns update response: {:?}", response);
    Ok(())
}

 #[derive(Serialize)]
struct DnsUpdateRequest {
    r#type: String,
    name: String,
    content: String,
    ttl: i32,
    proxied: bool,
}
 