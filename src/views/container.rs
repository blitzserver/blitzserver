use html_node::{text, html};
pub fn wrap_container(title: Option<&str>, meta: Option<html_node::Node>, contents: Option<html_node::Node>) -> html_node::Node {
    html! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <title>{text!("{}", title.unwrap_or("Blitzserver"))}</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <meta charset="utf-8" />
                { meta }
                <meta name="description" content="Dominions 5 hosting service"/>
                <link rel="stylesheet" href={text!("/styles.css?{}", env!("CARGO_PKG_VERSION"))} />
                <link rel="icon" href="/favicon.png" />
                <link rel="preload" href="/serpent-lay-front.webp" />
                <link rel="preload" href="/serpent-lay-back.webp" />
                <link rel="preload" href="/serpent-dancer.webp" />
                <link rel="preload" href="/serpent-croc.webp" />
            </head>
            <body>
                <div class="container {% block containerclass %}{% endblock %}">
                    <nav class={ text!("{}", if crate::frontend::utils::rand_bool() { "lizard-bg-alt"} else { "lizard-bg" } )}>
                        <span class="title">Blitzserver</span>
                        <a href="/games">games</a>
                        <a href="/maps">maps</a>
                        <a href="/mods">mods</a>
                        <a href="/help">about</a>
                        <span class="version"><a href={ text!("https://github.com/blitzserver/blitzserver/releases/tag/{}", env!("CARGO_PKG_VERSION")) }>
                            {text!("{}", env!("CARGO_PKG_VERSION"))}
                        </a></span>
                    </nav>
                    { contents }
                </div>
            </body>
        </html>

    }
}
