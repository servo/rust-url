use idna_mapping::*;
use std::char;

fn idna_mapped(mapping: &'static [u32]) -> Result<String, &'static str> {
  let mut ret = "".to_string();
  for c in mapping {
    match char::from_u32(*c) {
      Some(c) => ret.push(c),
      None => return Err("Disallowed character in mapping")
    }
  }
  return Ok(ret);
}

fn idna_deviation(codepoint: char, mapping: &'static [u32], transitional: bool) -> Result<String, &'static str> {
  if transitional {
    return idna_mapped(mapping);
  }
  return Ok(codepoint.to_string());
}

fn idna_disallowed_std3_valid(codepoint: char, use_std3_asciirules: bool) -> Result<String, &'static str> {
  if use_std3_asciirules {
    return Err("Dissallowed. Only valid in STD3");
  }
  return Ok(codepoint.to_string());
}

fn idna_disallowed_std3_mapped(mapping: &'static [u32], use_std3_asciirules: bool) -> Result<String, &'static str> {
  if use_std3_asciirules {
    return Err("Dissallowed. Mapped in STD3");
  }
  return idna_mapped(mapping);
}

// string, false, true
fn map_char(codepoint: char, use_std3_asciirules: bool, transitional: bool) -> Result<String, &'static str> {
  let mut min = 0;
  let mut max = TABLE.len() - 1;
  while max > min {
    let mid = (min + max) >> 1;
    if (codepoint as u32) > TABLE[mid].to {
      min = mid;
    } else if (codepoint as u32) < TABLE[mid].from {
      max = mid;
    } else {
      min = mid;
      max = mid;
    }
  }

  let mapping = TABLE[min].mapping;

  match TABLE[min].status {
    MappingStatus::valid => return Ok(codepoint.to_string()),
    MappingStatus::ignored => return Ok("".to_string()),
    MappingStatus::mapped => return idna_mapped(mapping),
    MappingStatus::deviation => return idna_deviation(codepoint, mapping, transitional),
    MappingStatus::disallowed => return Err("Dissallowed"),
    MappingStatus::disallowed_STD3_valid => return idna_disallowed_std3_valid(codepoint, use_std3_asciirules),
    MappingStatus::disallowed_STD3_mapped => return idna_disallowed_std3_mapped(mapping, use_std3_asciirules),
  }
}

pub fn domain_to_ascii(domain: &str) -> Result<String, &'static str> {
  let mut ret = "".to_string();
  for c in domain.chars() {
    match map_char(c, false, true) {
      Ok(mystr) => ret.push_str(&mystr),
      Err(x) => return Err(x)
    }
  }
  return Ok(ret);
}