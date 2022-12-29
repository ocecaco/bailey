use crate::heap_value::{Closure, HeapAddress, HeapValue, RefCountedHeapValue, Tuple};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Heap {
    memory: HashMap<HeapAddress, RefCountedHeapValue>,
    heap_next_address: HeapAddress,
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            memory: HashMap::new(),
            heap_next_address: HeapAddress(0),
        }
    }

    pub fn alloc(&mut self, heap_value: HeapValue) -> HeapAddress {
        let address = self.heap_next_address;
        self.heap_next_address = HeapAddress(self.heap_next_address.0 + 1);
        let refcounted = RefCountedHeapValue {
            refcount: 0,
            heap_value,
        };
        self.memory.insert(address, refcounted);
        address
    }

    pub fn deref(&self, heap_address: HeapAddress) -> &HeapValue {
        &self.memory[&heap_address].heap_value
    }

    pub fn deref_mut(&mut self, heap_address: HeapAddress) -> &mut HeapValue {
        &mut self
            .memory
            .get_mut(&heap_address)
            .expect("invalid pointer")
            .heap_value
    }

    pub fn inc_refcount(&mut self, heap_address: HeapAddress) {
        let refcounted = &mut self.memory.get_mut(&heap_address).expect("invalid pointer");
        refcounted.refcount += 1;
    }

    pub fn dec_refcount(&mut self, heap_address: HeapAddress) {
        let new_refcount = {
            let refcounted = &mut self.memory.get_mut(&heap_address).expect("invalid pointer");
            refcounted.refcount -= 1;
            refcounted.refcount
        };

        if new_refcount == 0 {
            self.free(heap_address);
        }
    }

    fn free(&mut self, heap_address: HeapAddress) {
        let destroying_value = self
            .memory
            .remove(&heap_address)
            .expect("attempt to free invalid pointer")
            .heap_value;

        match destroying_value {
            HeapValue::Int(_) => {}
            HeapValue::Bool(_) => {}
            HeapValue::Tuple(Tuple { field_values }) => {
                for addr in field_values {
                    self.dec_refcount(addr);
                }
            }
            HeapValue::Closure(Closure { environment, .. }) => {
                for addr in environment.values() {
                    self.dec_refcount(*addr);
                }
            }
        }
    }
}
