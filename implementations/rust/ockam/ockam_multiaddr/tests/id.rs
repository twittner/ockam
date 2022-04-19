use ockam_multiaddr::proto::{Dns, Tcp};
use ockam_multiaddr::{Code, MultiAddr, Protocol};
use quickcheck::{quickcheck, Arbitrary, Gen};
use rand::prelude::*;
use std::net::{Ipv4Addr, Ipv6Addr};

/// Newtype to implement `Arbitrary` for.
#[derive(Debug, Clone)]
struct Addr(MultiAddr);

#[derive(Debug, Copy, Clone)]
struct Proto(Code);

quickcheck! {
    fn to_str_from_str_prop(a: Addr) -> bool {
        to_str_from_str(a)
    }

    fn to_bytes_from_bytes_prop(a: Addr) -> bool {
        to_bytes_from_bytes(a)
    }

    fn mutate(p: Proto, ops: Vec<Op>) -> bool {
        let mut ma = MultiAddr::default();
        for o in ops {
            to_str_from_str(Addr(ma.clone()));
            to_bytes_from_bytes(Addr(ma.clone()));
            match o {
                Op::PopBack  => { ma.pop_back(); }
                Op::PopFront => { ma.pop_front(); }
                Op::DropLast => { ma.drop_last(); }
                Op::Last     => { ma.last(); }
                Op::PushBack => match p.0 {
                    Tcp::CODE => ma.push_back(Tcp(0)).unwrap(),
                    Dns::CODE => ma.push_back(Dns::new("localhost")).unwrap(),
                    _         => ma.push_back(Ipv4Addr::from([172,0,0,2])).unwrap()
                }
            }
        }
        true
    }
}

fn to_str_from_str(a: Addr) -> bool {
    let s = a.0.to_string();
    let b = MultiAddr::try_from(s.as_str()).unwrap();
    a.0 == b
}

fn to_bytes_from_bytes(a: Addr) -> bool {
    let b = MultiAddr::try_from(a.0.as_ref()).unwrap();
    a.0 == b
}

const PROTOS: &[Code] = &[Tcp::CODE, Dns::CODE, Ipv4Addr::CODE, Ipv6Addr::CODE];

impl Arbitrary for Proto {
    fn arbitrary(g: &mut Gen) -> Self {
        Proto(*g.choose(PROTOS).unwrap())
    }
}

impl Arbitrary for Addr {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut a = MultiAddr::default();
        for _ in 0..g.size() {
            match Proto::arbitrary(g).0 {
                Tcp::CODE => a.push_back(Tcp(u16::arbitrary(g))).unwrap(),
                Dns::CODE => a.push_back(Dns::new(gen_hostname())).unwrap(),
                Ipv4Addr::CODE => a.push_back(Ipv4Addr::arbitrary(g)).unwrap(),
                Ipv6Addr::CODE => a.push_back(Ipv6Addr::arbitrary(g)).unwrap(),
                _ => unreachable!(),
            }
        }
        Addr(a)
    }
}

#[derive(Debug, Copy, Clone)]
enum Op {
    PopBack,
    PushBack,
    PopFront,
    DropLast,
    Last,
}

impl Arbitrary for Op {
    fn arbitrary(g: &mut Gen) -> Self {
        *g.choose(&[
            Op::PopBack,
            Op::PushBack,
            Op::PopFront,
            Op::DropLast,
            Op::Last,
        ])
        .unwrap()
    }
}

fn gen_hostname() -> String {
    const LABEL: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz123456789_-";
    fn gen_label<R: Rng>(g: &mut R) -> String {
        let num: usize = g.gen_range(1..=23);
        String::from_iter(LABEL.chars().choose_multiple(g, num).into_iter())
    }
    let mut g = rand::thread_rng();
    let mut v = Vec::new();
    for _ in 1..=10 {
        v.push(gen_label(&mut g))
    }
    v.join(".")
}
