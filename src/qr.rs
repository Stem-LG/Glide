use qrcode::{Color, QrCode};

/// Eight pixels per module and an explicit four-module white quiet zone.
/// The desktop displays these exact pixels in a texture, independent of theme.
pub fn pixels(address: &str) -> anyhow::Result<(usize, Vec<u8>)> {
    let qr = QrCode::new(address.as_bytes())?;
    let side = (qr.width() + 8) * 8;
    let mut pixels = vec![255; side * side];
    for y in 0..qr.width() {
        for x in 0..qr.width() {
            if qr[(x, y)] == Color::Dark {
                for dy in 0..8 {
                    for dx in 0..8 {
                        pixels[((y + 4) * 8 + dy) * side + (x + 4) * 8 + dx] = 0;
                    }
                }
            }
        }
    }
    Ok((side, pixels))
}

#[cfg(test)]
mod tests {
    #[test]
    fn rendered_qr_decodes_to_connection_url() {
        let address = "http://192.168.0.252:8766";
        let (side, pixels) = super::pixels(address).unwrap();
        let mut decoder = quircs::Quirc::default();
        let code = decoder
            .identify(side, side, &pixels)
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(code.decode().unwrap().payload, address.as_bytes());
    }
}
