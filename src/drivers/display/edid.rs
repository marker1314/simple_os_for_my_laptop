//! Basic EDID parser (VESA EDID 1.3/1.4 subset)

pub struct Edid<'a> { pub raw: &'a [u8] }

impl<'a> Edid<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < 128 { return None; }
        if &data[0..8] != b"\x00\xFF\xFF\xFF\xFF\xFF\xFF\x00" { return None; }
        Some(Self { raw: &data[..128] })
    }

    pub fn preferred_timing(&self) -> Option<(u16,u16,u16)> {
        // Very simplified: use detailed timing descriptor at 54
        if self.raw.len() < 128 { return None; }
        let dtd = &self.raw[54..72];
        let pxclk = u16::from_le_bytes([dtd[0], dtd[1]]); // in 10 kHz
        if pxclk == 0 { return None; }
        let h_active = (((dtd[4] & 0xF0) as u16) << 4) | dtd[2] as u16;
        let v_active = (((dtd[7] & 0xF0) as u16) << 4) | dtd[5] as u16;
        Some((h_active, v_active, pxclk))
    }
}


