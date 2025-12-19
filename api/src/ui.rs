use dioxus::fullstack::Form;
use dioxus::prelude::*;
use dioxus::router::RouterConfig;
use crate::common::Credentials;
use crate::login;

#[rustfmt::skip]
#[derive(Clone, Routable)]
pub enum Route {
    #[route("/")]
    Home{},
    #[route("/login")]
    LoginPage{},
    #[route("/class/:class_name#:profil_name")]
    ClassPage{
        class_name: String,
        profil_name: u32
    }
}

#[component]
pub fn Home() -> Element {
    rsx! {
        //LoginBar {}
        div { "Home" }
    }
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> { config: || RouterConfig::default() }
    }
}

#[component]
pub fn ClassPage(class_name: String, profil_name: String) -> Element {
    rsx! {
        "class name: {class_name}",
        "profil name: {profil_name}"
    }
}

#[component]
pub fn LoginPage() -> Element {
    let mut fetch_login = use_action(login);
    rsx!(
        form {
            onsubmit: move |evt: FormEvent| async move {
                // Prevent the browser from navigating away.
                evt.prevent_default();

                // Extract the form values into our `LoginForm` struct. The `.parsed_values` method
                // is provided by Dioxus and works with any form element that has `name` attributes.
                let values: Credentials = evt.parsed_values()?;

                info!("trying to log");

                // Call our server function with the form values wrapped in `Form`. The `SetHeader`
                // response will set a cookie in the browser if the login is successful.
                if login(Form(values)).await? {
                    navigator().replace(Route::Home {});
                }

                info!("logged");
                Ok(())
            },
            input { r#type: "text", id: "name", name: "name" }
            label { "NOM Prenom" }
            input { r#type: "password", id: "password", name: "password" }
            label { "Mot de passe" }
            button { "Login" }
        }
    )
}