use super::{Buffer, Code, Codec, Protocol};
use crate::proto::{Dns, Tcp};
use crate::Error;
use core::fmt;
use unsigned_varint::decode;

pub struct StdCodec;

impl Codec for StdCodec {
    fn split_str<'a>(&self, _prefix: &str, input: &'a str) -> Result<(&'a str, &'a str), Error> {
        if let Some(p) = input.find('/') {
            Ok(input.split_at(p))
        } else {
            Ok((input, ""))
        }
    }

    fn split_bytes<'a>(&self, code: Code, input: &'a [u8]) -> Result<(&'a [u8], &'a [u8]), Error> {
        match code {
            #[cfg(feature = "std")]
            std::net::Ipv4Addr::CODE => {
                if input.len() < 4 {
                    return Err(Error::required_bytes(std::net::Ipv4Addr::CODE, 4));
                }
                Ok(input.split_at(4))
            }
            #[cfg(feature = "std")]
            std::net::Ipv6Addr::CODE => {
                if input.len() < 16 {
                    return Err(Error::required_bytes(std::net::Ipv6Addr::CODE, 16));
                }
                Ok(input.split_at(16))
            }
            Tcp::CODE => {
                if input.len() < 2 {
                    return Err(Error::required_bytes(Tcp::CODE, 2));
                }
                Ok(input.split_at(2))
            }
            Dns::CODE => {
                let (len, val) = decode::usize(input)?;
                if val.len() < len {
                    return Err(Error::required_bytes(Dns::CODE, len));
                }
                Ok(input.split_at((input.len() - val.len()) + len))
            }
            _ => Err(Error::unregistered(code)),
        }
    }

    fn is_valid_bytes(&self, code: Code, input: &[u8]) -> bool {
        match code {
            #[cfg(feature = "std")]
            std::net::Ipv4Addr::CODE => std::net::Ipv4Addr::read_bytes(input).is_ok(),
            #[cfg(feature = "std")]
            std::net::Ipv6Addr::CODE => std::net::Ipv6Addr::read_bytes(input).is_ok(),
            Tcp::CODE => Tcp::read_bytes(input).is_ok(),
            Dns::CODE => Dns::read_bytes(input).is_ok(),
            _ => false,
        }
    }

    fn transcode_str(&self, prefix: &str, value: &str, buf: &mut dyn Buffer) -> Result<(), Error> {
        match prefix {
            #[cfg(feature = "std")]
            std::net::Ipv4Addr::PREFIX => {
                std::net::Ipv4Addr::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            #[cfg(feature = "std")]
            std::net::Ipv6Addr::PREFIX => {
                std::net::Ipv6Addr::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Tcp::PREFIX => {
                Tcp::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            Dns::PREFIX => {
                Dns::read_str(value)?.write_bytes(buf);
                Ok(())
            }
            _ => Err(Error::unregistered_prefix(prefix)),
        }
    }

    fn transcode_bytes(
        &self,
        code: Code,
        value: &[u8],
        f: &mut fmt::Formatter,
    ) -> Result<(), Error> {
        match code {
            #[cfg(feature = "std")]
            std::net::Ipv4Addr::CODE => {
                std::net::Ipv4Addr::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            #[cfg(feature = "std")]
            std::net::Ipv6Addr::CODE => {
                std::net::Ipv6Addr::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Tcp::CODE => {
                Tcp::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            Dns::CODE => {
                Dns::read_bytes(value)?.write_str(f)?;
                Ok(())
            }
            _ => Err(Error::unregistered(code)),
        }
    }
}
