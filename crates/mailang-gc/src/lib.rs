#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcState {
    White,
    Gray,
    Black,
}

#[derive(Debug)]
pub struct GcObject<T> {
    pub value: T,
    pub state: GcState,
    pub ref_count: usize,
    pub marked: bool,
}

impl<T> GcObject<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            state: GcState::White,
            ref_count: 1,
            marked: false,
        }
    }

    pub fn inc_ref(&mut self) {
        self.ref_count += 1;
    }

    pub fn dec_ref(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count == 0
    }
}

pub struct Gc {
    objects: Vec<Box<dyn std::any::Any>>,
    threshold: usize,
    allocation_count: usize,
}

impl Gc {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            threshold: 1000,
            allocation_count: 0,
        }
    }

    pub fn allocate<T: 'static>(&mut self, value: T) -> usize {
        let index = self.objects.len();
        self.objects.push(Box::new(GcObject::new(value)));
        self.allocation_count += 1;

        if self.allocation_count >= self.threshold {
            self.collect();
            self.threshold = (self.threshold as f64 * 1.5) as usize;
        }

        index
    }

    pub fn collect(&mut self) {
        self.objects.retain(|obj| {
            if let Some(gc_obj) = obj.downcast_ref::<GcObject<i32>>() {
                gc_obj.ref_count > 0
            } else {
                true
            }
        });
        self.allocation_count = 0;
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

impl Default for Gc {
    fn default() -> Self {
        Self::new()
    }
}
