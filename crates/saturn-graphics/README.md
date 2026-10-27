# saturn-graphics

Graphics templates for Easy Edits Pro — motion graphics with keyframe animation (.sgt format).

## Features

- **TemplateInstance** — Parameterized template instances with bindings
- **Keyframe** — Cubic bezier interpolation with ease in/out tangents
- **Layers** — Text, Shape, Image, Video, Solid with transform stack
- **FCGT-compatible** — Cross-compatible with reference-editor graphics templates
- **Animation** — Position, scale, rotation, opacity, color keyframes

## Format (.sgt)

```json
{
  "version": 1,
  "name": "Lower Third",
  "width": 1920,
  "height": 1080,
  "duration": 5.0,
  "layers": [
    {
      "type": "text",
      "name": "Title",
      "content": "Breaking News",
      "font": "Inter",
      "font_size": 72,
      "color": "#ffffff",
      "transform": { "position": [960, 900], "scale": [1, 1], "rotation": 0 },
      "keyframes": [
        { "time": 0.0, "property": "opacity", "value": 0, "easing": "easeOut" },
        { "time": 0.5, "property": "opacity", "value": 1, "easing": "easeIn" },
        { "time": 4.5, "property": "opacity", "value": 1, "easing": "easeOut" },
        { "time": 5.0, "property": "opacity", "value": 0, "easing": "easeIn" }
      ]
    }
  ],
  "parameters": [
    { "name": "title_text", "type": "string", "default": "Breaking News" },
    { "name": "primary_color", "type": "color", "default": "#ff0000" }
  ]
}
```

## Usage

```rust
use saturn_graphics::{GraphicsTemplate, TemplateInstance, Keyframe, Easing};

let template = GraphicsTemplate::from_file("lower_third.sgt")?;
let mut instance = TemplateInstance::new(&template);
instance.bind("title_text", "Live at 5");
instance.bind("primary_color", "#ff0000");

// Render at specific time
let frame = instance.render_at(1.5)?; // Returns RGBA frame
```

## Running Tests

```bash
cargo test -p saturn-graphics
```