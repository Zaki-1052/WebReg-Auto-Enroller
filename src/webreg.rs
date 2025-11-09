use std::error::Error as StdError;
use webweg::wrapper::WebRegWrapper;
use log::info;
use crate::config::WebRegConfig;

pub async fn initialize_webreg(config: &WebRegConfig) -> Result<WebRegWrapper, Box<dyn StdError + Send + Sync>> {
    println!("Starting initialize_webreg");
    println!("Cookie length: {}", config.cookie.len());

    let wrapper = WebRegWrapper::builder()
        .with_cookies(&config.cookie)
        .try_build_wrapper()
        .ok_or("Failed to build WebReg wrapper")?;

    println!("Successfully built wrapper, attempting to associate term");

    let result = wrapper.associate_term(&config.term).await;
    match &result {
        Ok(_) => println!("Successfully associated term"),
        Err(e) => println!("Error associating term: {:?}", e),
    }

    result?;
    info!("Successfully initialized WebReg connection for term {}", config.term);

    Ok(wrapper)
}

pub async fn is_connection_valid(wrapper: &WebRegWrapper, term: &str) -> bool {
    match wrapper.associate_term(term).await {
        Ok(_) => true,
        Err(_) => false
    }
}
