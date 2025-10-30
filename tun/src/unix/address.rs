use std::io::{Error, ErrorKind};
use std::net::IpAddr;

use fehler::throws;

#[throws]
pub(crate) fn ensure_valid_ipv6_prefix(prefix_len: u8) {
    if prefix_len > 128 {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "IPv6 prefix length must be between 0 and 128",
        ))?;
    }
}

#[cfg_attr(not(any(test, target_vendor = "apple")), allow(dead_code))]
#[throws]
pub(crate) fn ipv6_prefix_octets(prefix_len: u8) -> [u8; 16] {
    ensure_valid_ipv6_prefix(prefix_len)?;

    let mut octets = [0u8; 16];
    for bit in 0..prefix_len {
        let idx = (bit / 8) as usize;
        let offset = (bit % 8) as u8;
        octets[idx] |= 0x80 >> offset;
    }

    octets
}

#[cfg_attr(not(any(test, target_vendor = "apple")), allow(dead_code))]
pub(crate) fn parse_addr_spec(spec: &str) -> Result<Option<(IpAddr, Option<u8>)>, Error> {
    let (addr_str, prefix) = match spec.split_once('/') {
        Some((addr, prefix)) => (addr, Some(prefix)),
        None => (spec, None),
    };

    let addr: IpAddr = match addr_str.parse() {
        Ok(addr) => addr,
        Err(_) => return Ok(None),
    };

    let prefix_len = if let Some(prefix) = prefix {
        let parsed = prefix
            .parse::<u8>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "Invalid prefix length"))?;
        ensure_valid_ipv6_prefix(parsed)?;
        Some(parsed)
    } else {
        None
    };

    Ok(Some((addr, prefix_len)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn parse_ipv4_without_prefix() {
        let parsed = parse_addr_spec("192.0.2.1").expect("parse succeeds");
        assert_eq!(
            parsed,
            Some((IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), None))
        );
    }

    #[test]
    fn parse_ipv6_with_prefix() {
        let parsed = parse_addr_spec("2001:db8::1/64").expect("parse succeeds");
        assert_eq!(
            parsed,
            Some((
                IpAddr::V6("2001:db8::1".parse::<Ipv6Addr>().unwrap()),
                Some(64),
            ))
        );
    }

    #[test]
    fn parse_invalid_addr_returns_none() {
        assert_eq!(parse_addr_spec("not-an-ip").unwrap(), None);
    }

    #[test]
    fn parse_invalid_prefix_string_errors() {
        assert!(parse_addr_spec("::1/not-a-number").is_err());
    }

    #[test]
    fn parse_prefix_out_of_range_errors() {
        assert!(parse_addr_spec("::1/129").is_err());
    }

    #[test]
    fn ensure_valid_ipv6_prefix_accepts_bounds() {
        ensure_valid_ipv6_prefix(0).expect("zero prefix is allowed");
        ensure_valid_ipv6_prefix(128).expect("max prefix is allowed");
    }

    #[test]
    fn ensure_valid_ipv6_prefix_rejects_invalid() {
        assert!(ensure_valid_ipv6_prefix(129).is_err());
    }

    #[test]
    fn ipv6_prefix_octets_zero_prefix() {
        assert_eq!(ipv6_prefix_octets(0).unwrap(), [0u8; 16]);
    }

    #[test]
    fn ipv6_prefix_octets_sets_bits_correctly() {
        let mask = ipv6_prefix_octets(65).unwrap();
        assert_eq!(mask[0..8], [0xFF; 8]);
        assert_eq!(mask[8], 0x80);
        assert_eq!(mask[9..], [0u8; 7]);
    }
}
