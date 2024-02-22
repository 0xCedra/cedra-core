use aptos_oidb_pepper_common::{
    asymmetric_encryption::{scheme1::Scheme, AsymmetricEncryption},
    jwt, nonce_derivation,
    nonce_derivation::NonceDerivationScheme,
    pepper_pre_image_derivation,
    pepper_pre_image_derivation::PepperPreImageDerivation,
    vuf,
    vuf::VUF,
    EncryptionPubKey, PepperRequest, PepperResponse, VUFVerificationKey,
};
use rand::thread_rng;
use std::{fs, io::stdin};

const TEST_JWT: &str = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0.eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiJFVVRhSE9HdDcwRTNxbk9QMUJibnUzbE03QjR5TTdzaHZTb1NvdXF1VVJ3IiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9.CEgO4S7hRgASaINsGST5Ygtl_CY-mUn2GaQ6d7q9q1eGz1MjW0o0yusJQDU6Hi1nDfXlNSvCF2SgD9ayG3uDGC5-18H0AWo2QgyZ2rC_OUa36RCTmhdo-i_H8xmwPxa3yHZZsGC-gJy_vVX-rfMLIh-JgdIFFIzGVPN75MwXLP3bYUaB9Lw52g50rf_006Qg5ubkZ70I13vGUTVbRVWanQIN69naFqHreLCjVsGsEBVBoUtexZw6Ulr8s0VajBpcTUqlMvbvqMfQ33NXaBQYvu3YZivpkus8rcG_eAMrFbYFY9AZF7AaW2HUaYo5QjzMQDsIA1lpnAcOW3GzWvb0vw";

fn read_line_from_stdin() -> String {
    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();
    line
}

fn get_pepper_service_url() -> String {
    match std::env::var("OIDB_PEPPER_TEST_CLIENT__SERVICE_URL") {
        Ok(val) => {
            println!();
            println!(
                "Pepper service url found from envvar OIDB_PEPPER_TEST_CLIENT__SERVICE_URL: {}",
                val
            );
            val
        },
        Err(_) => {
            println!();
            println!("Pepper service url not found from envvar OIDB_PEPPER_SERVICE_URL.");
            println!("Enter the URL of the targeted pepper service deployment (default: http://localhost:8000):");
            let raw = read_line_from_stdin().trim().to_string();
            if raw.is_empty() {
                "http://localhost:8000".to_string()
            } else {
                raw
            }
        },
    }
}

fn get_jwt_or_path() -> String {
    println!();
    println!(
        "Enter the JWT token (defaults to test token), or a text file path that contains the JWT:"
    );
    let user_input = read_line_from_stdin().trim().to_string();
    if !user_input.is_empty() {
        user_input
    } else {
        println!("Using the test JWT token");
        TEST_JWT.to_string()
    }
}

#[tokio::main]
async fn main() {
    let mut rng = thread_rng();
    println!();
    println!("Starting an interaction with aptos-oidb-pepper-service.");
    let url = get_pepper_service_url();
    println!();
    let vuf_vrfy_key_url = format!("{url}/vuf-pub-key");
    println!();
    println!(
        "Action 1: fetch its verification key with a GET request to {}",
        vuf_vrfy_key_url
    );
    let client = reqwest::Client::new();
    let response = client
        .get(vuf_vrfy_key_url)
        .send()
        .await
        .unwrap()
        .json::<VUFVerificationKey>()
        .await
        .unwrap();
    println!();
    println!(
        "response_json={}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    let VUFVerificationKey {
        scheme_name,
        payload_hexlified,
    } = response;
    assert_eq!("Scheme0", scheme_name.as_str());
    let vuf_pk_bytes = hex::decode(payload_hexlified).unwrap();

    println!();
    println!(
        "Action 2: generate a {} ephemeral key pair.",
        Scheme::scheme_name()
    );
    let (sk, pk) = Scheme::key_gen(&mut rng);
    println!("esk_hexlified={}", hex::encode(&sk));
    println!("epk_hexlified={}", hex::encode(&pk));

    println!();
    println!("Action 3: generate some random bytes as a blinder.");
    let blinder: [u8; 31] = [0u8; 31];
    println!("blinder_hexlified={}", hex::encode(blinder));

    println!();
    println!("Action 4: decide an expiry unix time.");
    let expiry_time_sec = 2000000000;
    println!("expiry_time_sec={}", expiry_time_sec);

    let epk: [u8; 34] = [
        0, 32, 208, 74, 178, 50, 116, 43, 180, 171, 58, 19, 104, 189, 70, 21, 228, 230, 208, 34,
        74, 183, 26, 1, 107, 175, 133, 32, 163, 50, 201, 119, 135, 55,
    ];

    println!();
    println!("Action 5: compute a nonce as a hash of the EPK, the blinder and the expiry time.");
    let nonce_pre_image = nonce_derivation::scheme1::PreImage {
        epk: epk.to_vec(),
        expiry_time_sec,
        blinder: blinder.to_vec(),
    };
    let nonce = nonce_derivation::scheme1::Scheme::derive_nonce(&nonce_pre_image);
    let nonce_str = hex::encode(nonce);
    println!("nonce_string={}", nonce_str);
    println!();
    println!("Action 6: request a JWT with this nonce. Below are generated example that uses Google OAuth 2.0 Playground:");
    println!("6.1: Go to https://accounts.google.com/o/oauth2/v2/auth/oauthchooseaccount?redirect_uri=https%3A%2F%2Fdevelopers.google.com%2Foauthplayground&prompt=consent&response_type=code&client_id=407408718192.apps.googleusercontent.com&scope=profile&access_type=offline&service=lso&o2v=2&theme=glif&flowName=GeneralOAuthFlow&nonce={nonce_str}");
    println!("6.2: Sign in as requested by the web UI");
    println!("6.3: Once you are signed in to 'OAuth 2.0 Playground' and see a blue button called 'Exchange authorization code for tokens', click it");
    println!("6.4: You should see some response showing up. Take the value of the field 'id_token' (exclude the double-quotes) and save it to a file");
    let jwt_or_path = get_jwt_or_path();
    let jwt = match fs::read_to_string(jwt_or_path.clone()) {
        Ok(raw_str) => raw_str.trim().to_string(),
        Err(_) => jwt_or_path,
    };

    let pepper_request = PepperRequest {
        jwt: jwt.clone(),
        overriding_aud: None,
        ephem_pub_key_hexlified: hex::encode(epk),
        enc_pub_key: EncryptionPubKey {
            scheme_name: Scheme::scheme_name(),
            payload_hexlified: hex::encode(pk),
        },
        expiry_time_sec,
        blinder_hexlified: hex::encode(blinder),
        uid_key: None,
    };
    println!();
    println!(
        "Request pepper with a POST to {} and the body being {}",
        url,
        serde_json::to_string_pretty(&pepper_request).unwrap()
    );
    let raw_response = client.post(url).json(&pepper_request).send().await.unwrap();
    let pepper_response = raw_response.json::<PepperResponse>().await.unwrap();
    println!();
    println!(
        "pepper_service_response={}",
        serde_json::to_string_pretty(&pepper_response).unwrap()
    );
    let PepperResponse::OK {
        pepper_encrypted_hexlified,
    } = pepper_response
    else {
        panic!()
    };
    let pepper_encrypted = hex::decode(pepper_encrypted_hexlified).unwrap();
    let pepper_bytes = Scheme::dec(sk.as_slice(), pepper_encrypted.as_slice()).unwrap();
    println!();
    println!("Decrypt the pepper using the ephemeral private key.");
    println!("pepper_bytes={:?}", pepper_bytes);
    let claims = jwt::parse(jwt.as_str()).unwrap();
    println!();
    println!("Verify the pepper against the server's verification key and part of the JWT.");
    let pepper_input_source = pepper_pre_image_derivation::scheme1::Source {
        iss: claims.claims.iss.clone(),
        uid_key: "sub".to_string(),
        uid_val: claims.claims.sub.clone(),
        aud: claims.claims.aud.clone(),
    };
    let pepper_input = pepper_pre_image_derivation::scheme1::Scheme::derive(&pepper_input_source);
    vuf::scheme0::Scheme::verify(&vuf_pk_bytes, &pepper_input, &pepper_bytes, &[]).unwrap();
    println!();
    println!("Pepper verification succeeded!");
}
