extern crate ring;

use std::thread;
use std::sync::mpsc::channel;
use std::time::{Instant};

use ring::channel::channel as ring_channel;

fn main(){
    let max = 1_000_000;

    let t = Instant::now();
    chan1(max);
    eprintln!("chan {:?}", Instant::now() - t);
    
    let t = Instant::now();
    ring1(max);
    eprintln!("ring {:?}", Instant::now() - t);

    let t = Instant::now();
    chan2(max);
    eprintln!("chan {:?}", Instant::now() - t);
    
    let t = Instant::now();
    ring2(max);
    eprintln!("ring {:?}", Instant::now() - t);
}

#[allow(dead_code)]
fn chan1(max: usize){

    let (tx, rx) = channel::<u64>();

    let handle = thread::spawn(move ||{
        for i in 0..max*2 {
            tx.send(i as u64).unwrap();
        }
    });

    let mut sum = 0u64;    
    for _ in 0..max*2 {
        'inner: loop {

        match rx.recv().ok() {
            Some(v) => {
                sum += v as u64;
                break 'inner;
            },
            None => continue,
        }
        }
    }

    println!("chan(1) sum = {}", sum);
    handle.join().unwrap();
}

#[allow(dead_code)]
fn chan2(max: usize){

    let (tx0, rx) = channel::<u64>();

    let tx1 = tx0.clone();

    let handle0 = thread::spawn(move ||{
        for i in 0..max {
            tx0.send(i as u64).unwrap();
        }
    });

    let handle1 = thread::spawn(move ||{
        for i in 0..max {
            tx1.send(i as u64).unwrap();
        }
    });

    let mut sum = 0u64;    
    for _ in 0..max*2 {
        'inner: loop {

        
        match rx.recv().ok() {
            Some(v) => {
                sum += v as u64;
                break 'inner;
            },
            None => continue,
        }
        }
    }

    println!("chan(2) sum = {}", sum);
    handle0.join().unwrap();
    handle1.join().unwrap();
}


#[allow(dead_code)]
fn ring1 (max: usize) {
    let (tx, rx) = ring_channel::<u64>(8);

    let handle = thread::spawn(move ||{
        for i in 0..max*2 {
            tx.send(i as u64);
        }
    });

    let mut sum = 0u64;    
    for _ in 0..max*2 {
        sum += rx.recv();
    }
    eprintln!("ring(1) sum = {}", sum);
    handle.join().unwrap();
}

#[allow(dead_code)]
fn ring2 (max: usize) {
    let (tx0, rx) = ring_channel::<u64>(8);
    let tx1 = rx.clone();

    let handle0 = thread::spawn(move ||{
        for i in 0..max {
            tx0.send(i as u64);
        }
    });

    let handle1 = thread::spawn(move ||{
        for i in 0..max {
            tx1.send(i as u64);
        }
    });

    let mut sum = 0u64;    
    for _ in 0..max*2 {
        sum += rx.recv();
    }
    
    eprintln!("ring(2) sum = {}", sum);
    handle0.join().unwrap();
    handle1.join().unwrap();
}
