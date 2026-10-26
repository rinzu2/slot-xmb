//! Reads the handheld's background pictures from `System/xmb/` on the card, if the owner has
//! copied them there (see `slot_ui::xmb_ground_faces`).

use std::path::Path;

use slot_ui::CartFace;

pub fn load(root: &Path) -> Vec<CartFace> {
    let dir = root.join("System").join("xmb");
    let Ok(others) = std::fs::read(dir.join(slot_ui::XMB_GROUND_OTHERS)) else {
        return Vec::new();
    };
    let Some(months) = slot_ui::XMB_GROUND_MONTHS
        .iter()
        .find_map(|name| std::fs::read(dir.join(name)).ok())
    else {
        return Vec::new();
    };
    let faces = slot_ui::xmb_ground_faces(&others, &months);
    if faces.is_empty() {
        eprintln!("slot: xmb: System/xmb pictures were not as expected, using the fitted colours");
    }
    faces
}

/// The picture for `colour` (1 to 34), if there are pictures.
pub fn face(faces: &[CartFace], colour: u32) -> Option<&CartFace> {
    faces.get((colour as usize).checked_sub(1)?)
}
