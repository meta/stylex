use phf::phf_map;

pub static PSEUDO_CLASS_PRIORITIES: phf::Map<&'static str, &'static f64> = phf_map! {
  ":is" => &40.0,
  ":where" => &40.0,
  ":not" => &40.0,
  ":has" => &45.0,
  ":dir" => &50.0,
  ":lang" => &51.0,
  ":first-child" => &52.0,
  ":first-of-type" => &53.0,
  ":last-child" => &54.0,
  ":last-of-type" => &55.0,
  ":only-child" => &56.0,
  ":only-of-type" => &57.0,

  ":nth-child" => &60.0,
  ":nth-last-child" => &61.0,
  ":nth-of-type" => &62.0,
  ":nth-last-of-type" => &63.0, // "nth-last-of-type" is the same priority as "nth-of-typ.0e
  ":empty" => &70.0,

  ":link" => &80.0,
  ":any-link" => &81.0,
  ":local-link" => &82.0,
  ":target-within" => &83.0,
  ":target" => &84.0,
  ":visited" => &85.0,

  ":enabled" => &91.0,
  ":disabled" => &92.0,
  ":required" => &93.0,
  ":optional" => &94.0,
  ":read-only" => &95.0,
  ":read-write" => &96.0,
  ":placeholder-shown" => &97.0,
  ":in-range" => &98.0,
  ":out-of-range" => &99.0,

  ":default" => &100.0,
  ":checked" => &101.0,
  ":indeterminate" => &101.0,
  ":blank" => &102.0,
  ":valid" => &103.0,
  ":invalid" => &104.0,
  ":user-invalid" => &105.0,

  ":autofill" => &110.0,

  ":picture-in-picture" => &120.0,
  ":modal" => &121.0,
  ":fullscreen" => &122.0,
  ":paused" => &123.0,
  ":playing" => &124.0,
  ":current" => &125.0,
  ":past" => &126.0,
  ":future" => &127.0,

  ":hover" => &130.0,
  ":focusWithin" => &140.0,
  ":focus" => &150.0,
  ":focusVisible" => &160.0,
  ":active" => &170.0,
};

pub static AT_RULE_PRIORITIES: phf::Map<&'static str, &'static f64> = phf_map! {
  "@supports" => &30.0,
  "@media" => &200.0,
  "@container" => &300.0,
};

pub static PSEUDO_ELEMENT_PRIORITY: f64 = 5000.0;
