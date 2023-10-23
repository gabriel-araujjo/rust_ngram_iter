use ringbuffer::ConstGenericRingBuffer;

pub enum State<T, const N: usize> {
    Start,
    Middle(ConstGenericRingBuffer<T, N>),
    End,
}
