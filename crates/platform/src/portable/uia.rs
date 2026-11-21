//! UI element snapping result types and pure helpers, shared by every backend.

// `clip_to_ancestors` is only called by the Windows UIA walker so far.
#![cfg_attr(not(windows), allow(dead_code))]

use stinkshot_core::{PxPoint, PxRect};

#[derive(Clone, Debug, PartialEq)]
pub struct SnapResult {
    pub point: PxPoint,
    /// Element rects from innermost to the top-level window, clipped to the window rect,
    /// deduplicated, each with at least 4x4 px. The UI shows `rects[0]` and lets the user
    /// cycle outward with the mouse wheel.
    pub rects: Vec<PxRect>,
    /// What each entry of `rects` is (same length and order). Added for the overlay's
    /// "which element is highlighted" label.
    pub labels: Vec<ElementLabel>,
}

/// UIA control type and name of a snapped element.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElementLabel {
    /// Readable control type ("Button", "List Item", "Window", ...).
    pub control: &'static str,
    /// The element's UIA name, possibly empty.
    pub name: String,
}

/// Readable name for a UIA control type id (`UIA_*ControlTypeId`, 50000..=50040).
pub fn control_type_name(id: i32) -> &'static str {
    const NAMES: [&str; 41] = [
        "Button",
        "Calendar",
        "Check Box",
        "Combo Box",
        "Edit",
        "Hyperlink",
        "Image",
        "List Item",
        "List",
        "Menu",
        "Menu Bar",
        "Menu Item",
        "Progress Bar",
        "Radio Button",
        "Scroll Bar",
        "Slider",
        "Spinner",
        "Status Bar",
        "Tab",
        "Tab Item",
        "Text",
        "Tool Bar",
        "Tooltip",
        "Tree",
        "Tree Item",
        "Custom",
        "Group",
        "Thumb",
        "Data Grid",
        "Data Item",
        "Document",
        "Split Button",
        "Window",
        "Pane",
        "Header",
        "Header Item",
        "Table",
        "Title Bar",
        "Separator",
        "Semantic Zoom",
        "App Bar",
    ];
    usize::try_from(id - 50000)
        .ok()
        .and_then(|i| NAMES.get(i).copied())
        .unwrap_or("Element")
}

/// Clip each rect of an outermost-first element chain to the clipped rect of its parent.
/// `None` once a level no longer overlaps its parent (and for everything inside it).
pub(crate) fn clip_to_ancestors(chain: &[PxRect]) -> Vec<Option<PxRect>> {
    let mut acc: Option<PxRect> = None;
    chain
        .iter()
        .enumerate()
        .map(|(i, r)| {
            acc = if i == 0 {
                Some(*r)
            } else {
                acc.and_then(|a| r.intersect(&a))
            };
            acc
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_is_clipped_to_each_ancestor() {
        let window = PxRect::new(0, 0, 800, 600);
        // A list viewport and an item scrolled half out of it.
        let list = PxRect::new(100, 100, 300, 200);
        let item = PxRect::new(100, 250, 300, 100);
        let inner = PxRect::new(120, 260, 50, 80);
        let c = clip_to_ancestors(&[window, list, item, inner]);
        assert_eq!(c[0], Some(window));
        assert_eq!(c[1], Some(list));
        assert_eq!(c[2], Some(PxRect::new(100, 250, 300, 50)));
        assert_eq!(c[3], Some(PxRect::new(120, 260, 50, 40)));
        // A level outside its parent ends the chain.
        let c = clip_to_ancestors(&[window, PxRect::new(900, 0, 10, 10), list]);
        assert_eq!(c, vec![Some(window), None, None]);
    }

    #[test]
    fn control_type_names() {
        assert_eq!(control_type_name(50000), "Button");
        assert_eq!(control_type_name(50007), "List Item");
        assert_eq!(control_type_name(50032), "Window");
        assert_eq!(control_type_name(50033), "Pane");
        assert_eq!(control_type_name(50040), "App Bar");
        assert_eq!(control_type_name(50041), "Element");
        assert_eq!(control_type_name(0), "Element");
    }
}
