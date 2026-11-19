use phf::phf_map;

// Using MDN data as a source of truth to populate the above sets
// by group in alphabetical order:

pub static NUMBER_PROPERTY_SUFFIXIES: phf::Map<&'static str, &'static str> = phf_map! {
  "animationDelay"=> "ms",
  "animationDuration"=> "ms",
  "transitionDelay"=> "ms",
  "transitionDuration"=> "ms",
  "voiceDuration"=> "ms",
};
