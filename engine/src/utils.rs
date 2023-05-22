/// # Examples
/// ```rust
/// let mut field = vrt_engine::world::BitField::ZERO;
///
/// field.set(1, 1, 1);
/// assert_eq!(field.raw(), 0b00000000_00000010);
///
/// field.set(1, 1, 2);
/// assert_eq!(a.raw(), 0b00000000_00000110);
///
/// field.set(0b101, 3, 5);
/// assert_eq!(field.raw(), 0b00000000_10100110);
///
/// field.set(0b11011101, 8, 8);
/// assert_eq!(field.raw(), 0b11011101_10100110);
///
/// assert_eq!(field.get(1, 0), 0);
/// assert_eq!(field.get(1, 1), 1);
/// assert_eq!(field.get(2, 0), 0b10);
/// assert_eq!(field.get(2, 1), 0b11);
///
/// field.set(0, 8, 0);
/// assert_eq!(field.raw(), 0b11011101_00000000);
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BitField(u32);
impl BitField {
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub fn set(&mut self, data: u32, len: u32, offset: u32) {
        let mask = !(!0 << len) << offset;
        self.0 = (self.0 & !mask) | (data << offset);
    }

    #[inline(always)]
    pub fn get(self, len: u32, offset: u32) -> u32 {
        let mask = !(!0 << len) << offset;
        (self.0 & mask) >> offset
    }
}
