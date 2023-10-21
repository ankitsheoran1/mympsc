use::std::sync::{Arc, Mutex, Condvar};
use::std::collections::VecDeque;

pub struct Sender<T> {
   shared: Arc<Shared<T>>
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders += 1;
        drop(inner);
        Sender {
            shared: Arc::clone(&self.shared)
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders -= 1;
        let last = inner.senders == 0;
        if last {
            self.shared.available.notify_one();
        }
        
    }
}

pub struct Recevier<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(&mut self, t:T){
        let mut inner = self.shared.inner.lock().unwrap();
        inner.queue.push_back(t);
        dbg!(inner.senders);
      // dropping bfore notify as other thread wakes up they can immedialtely take lock
        drop(inner);
        self.shared.available.notify_one();

    }
}

impl<T> Recevier<T> {
    pub fn recv(&mut self) -> Option<T>  {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
        
        match inner.queue.pop_front() {
            Some(t) => return Some(t),
            None if inner.senders == 0 => return None,
            None => {
                // we would block 
                inner = self.shared.available.wait(inner).unwrap();
            }
        }

    }
}
}

struct Shared<T> {
    inner:Mutex<Inner<T>>,
    // should outside mutex - if u currently holding mutex, u realize other people wake up , 
    // those thread may wake up but they found lock not free and again sleep so no thread actually wake up and its deadlock :: damn
    available: Condvar,
}

struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize,
}

pub fn channel<T>() -> (Sender<T>, Recevier<T>) {
    let inner = Inner {
        queue: VecDeque::default(),
        senders: 1,
    };
    let shared = Shared {
        inner: Mutex::new(inner),
        available: Condvar::new(),
    };
    let shared = Arc::new(shared);
    (
        Sender {
            shared: shared.clone(),
        },
        Recevier {
            shared: shared.clone(),
        },
    )


}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_pong() {
        let (mut tx, mut rx) = channel();
        tx.send(42);
        assert_eq!(rx.recv(), Some(42));
    }

    #[test]
    fn closed_tx() {
        let (tx, mut rx) = channel::<()>();
        // let _ = tx;
        drop(tx);
        assert_eq!(rx.recv(), None);
    }

    #[test]
    fn closed_rx() {
        let (mut tx, rx) = channel();
        drop(rx);
        tx.send(2);

    }

    #[test]
    fn multiple_xx() {
        let (mut tx, rx) = channel();
        let mut handles = Vec::new();
        tx.send(1);
        tx.send(2);
        let mut rx1 = Recevier {
            shared: Arc::clone(&rx.shared),
        };
        handles.push(std::thread::spawn(move || {
            let recv_val = rx1.recv();
            println!("{:#?}", recv_val);
            assert_eq!(Some(1), recv_val);
        }));
        let mut rx2 = Recevier {
            shared: Arc::clone(&rx.shared),
        };
        handles.push(std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let recv_val = rx2.recv();
            println!("{:#?}", recv_val);
            assert_eq!(Some(2), recv_val);
        }));
        for handle in handles {
            handle.join().unwrap();
        }
    }
}

