use phf::phf_map;

pub static CURSOR_FLIP: phf::Map<&'static str, &'static str> = phf_map! {
  "e-resize" => "w-resize",
  "w-resize" => "e-resize",
  "ne-resize" => "nw-resize",
  "nesw-resize" => "nwse-resize",
  "nw-resize" => "ne-resize",
  "nwse-resize" => "nesw-resize",
  "se-resize" => "sw-resize",
  "sw-resize" => "se-resize",
};
