use audiotab::buffers::BufferPool;

#[test]
fn test_buffer_pool_get_and_return() {
    let pool = BufferPool::new(1024);

    let buffer1 = pool.get();
    assert!(buffer1.capacity() >= 1024);

    drop(buffer1); // Returns to pool

    let buffer2 = pool.get();
    // Should reuse the buffer
    assert!(buffer2.capacity() >= 1024);
}

#[test]
fn test_buffer_pool_multiple_buffers() {
    let pool = BufferPool::new(512);

    let buf1 = pool.get();
    let buf2 = pool.get();
    let buf3 = pool.get();

    // All should have correct capacity
    assert!(buf1.capacity() >= 512);
    assert!(buf2.capacity() >= 512);
    assert!(buf3.capacity() >= 512);

    drop(buf1);
    drop(buf2);
    drop(buf3);

    // Pool should have 3 buffers available
    let buf4 = pool.get();
    assert!(buf4.capacity() >= 512);
}

#[test]
fn test_buffer_pool_concurrent() {
    use std::sync::Arc;
    use std::thread;

    let pool = Arc::new(BufferPool::new(256));
    let mut handles = vec![];

    for _ in 0..10 {
        let pool_clone = pool.clone();
        let handle = thread::spawn(move || {
            let _buffer = pool_clone.get();
            thread::sleep(std::time::Duration::from_millis(10));
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
