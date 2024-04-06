use html_node::{text, Node, html};
pub fn map_thumb(map_id: i32, planes: Vec<Option<i32>>, width: u32, font_size: u32) -> Node {
    let id: u32 = rand::random();
    html! {
        <div class="map-thumb">
            <div class="map-thumb-radios">
            {planes.iter().enumerate().map(|(i, prov_count)| {
                let wrapped = (i + 1) % planes.len();
                html! {
                <input type="radio" name={format!("map-radio-{id}")} id={format!("map-radio-{id}-{i}")} { if i == 0 { "checked" } else { "" } } />
                <label for={format!("map-radio-{id}-{wrapped}")}>
                    <img class="map-thumb-img" src={format!("/maps/{map_id}/planes/{i}/thumb.webp?width={width}")} />
                    <p style={format!("font-size:{font_size}px")}>{ prov_count.map(|p| text!("{p}")).unwrap_or(text!("")) }</p>
                </label>
                }
            })}
            </div>
        </div>
    }
}
