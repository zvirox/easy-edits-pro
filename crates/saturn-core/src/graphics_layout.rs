//! Pure geometry for Essential Graphics align/distribute (Phase 5).
//!
//! All maths operate on transform boxes in project pixels and never mutate
//! entity state — the engine commands apply the returned positions.

/// Axis-aligned bounding box of a graphic's transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphicBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignMode {
    Left,
    HCenter,
    Right,
    Top,
    VCenter,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DistributeAxis {
    Horizontal,
    Vertical,
}

impl AlignMode {
    pub fn parse(mode: &str) -> Option<Self> {
        match mode {
            "left" => Some(Self::Left),
            "hcenter" | "center_x" => Some(Self::HCenter),
            "right" => Some(Self::Right),
            "top" => Some(Self::Top),
            "vcenter" | "middle" | "center_y" => Some(Self::VCenter),
            "bottom" => Some(Self::Bottom),
            _ => None,
        }
    }
}

impl DistributeAxis {
    pub fn parse(axis: &str) -> Option<Self> {
        match axis {
            "horizontal" | "x" => Some(Self::Horizontal),
            "vertical" | "y" => Some(Self::Vertical),
            _ => None,
        }
    }
}

/// Aligns each box inside the union bounds of the selection. Returns the new
/// origin (`x` or `y`) per input index, in input order.
pub fn align_boxes(boxes: &[GraphicBox], mode: AlignMode) -> Vec<f64> {
    let Some(first) = boxes.first() else {
        return Vec::new();
    };
    let min_x = boxes.iter().map(|b| b.x).fold(first.x, f64::min);
    let max_x = boxes
        .iter()
        .map(|b| b.x + b.width)
        .fold(first.x + first.width, f64::max);
    let min_y = boxes.iter().map(|b| b.y).fold(first.y, f64::min);
    let max_y = boxes
        .iter()
        .map(|b| b.y + b.height)
        .fold(first.y + first.height, f64::max);

    boxes
        .iter()
        .map(|box_| match mode {
            AlignMode::Left => min_x,
            AlignMode::HCenter => min_x + (max_x - min_x - box_.width) / 2.0,
            AlignMode::Right => max_x - box_.width,
            AlignMode::Top => min_y,
            AlignMode::VCenter => min_y + (max_y - min_y - box_.height) / 2.0,
            AlignMode::Bottom => max_y - box_.height,
        })
        .collect()
}

/// New origin per box for even-gap distribution along one axis. The first and
/// last boxes (in position order) stay where they are; with fewer than three
/// boxes nothing moves.
pub fn distribute_boxes(boxes: &[GraphicBox], axis: DistributeAxis) -> Vec<f64> {
    if boxes.len() < 3 {
        return boxes.iter().map(|b| b.x).collect();
    }
    let origin = |box_: &GraphicBox| match axis {
        DistributeAxis::Horizontal => box_.x,
        DistributeAxis::Vertical => box_.y,
    };
    let extent = |box_: &GraphicBox| match axis {
        DistributeAxis::Horizontal => box_.width,
        DistributeAxis::Vertical => box_.height,
    };
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|left, right| {
        let (a, b) = (origin(&boxes[*left]), origin(&boxes[*right]));
        a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
    });

    let first = boxes[order[0]];
    let last = boxes[order[order.len() - 1]];
    let (first_start, last_end) = match axis {
        DistributeAxis::Horizontal => (first.x, last.x + last.width),
        DistributeAxis::Vertical => (first.y, last.y + last.height),
    };
    let total_extent: f64 = boxes.iter().map(|b| extent(b)).sum();
    let total_span = last_end - first_start;
    let gap = if boxes.len() > 1 {
        (total_span - total_extent) / (boxes.len() - 1) as f64
    } else {
        0.0
    };

    let mut positions = vec![0.0; boxes.len()];
    let mut cursor = first_start;
    for index in order {
        positions[index] = cursor;
        cursor += extent(&boxes[index]) + gap.max(0.0);
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_(x: f64, y: f64, width: f64, height: f64) -> GraphicBox {
        GraphicBox { x, y, width, height }
    }

    #[test]
    fn align_left_lines_up_min_x() {
        let boxes = [box_(10.0, 0.0, 40.0, 10.0), box_(50.0, 5.0, 20.0, 8.0)];
        let result = align_boxes(&boxes, AlignMode::Left);
        assert_eq!(result, vec![10.0, 10.0]);
    }

    #[test]
    fn align_right_targets_union_right_edge() {
        let boxes = [box_(10.0, 0.0, 40.0, 10.0), box_(50.0, 5.0, 20.0, 8.0)];
        let result = align_boxes(&boxes, AlignMode::Right);
        assert_eq!(result, vec![30.0, 50.0]);
    }

    #[test]
    fn align_vcenter_centers_within_union() {
        let boxes = [box_(0.0, 0.0, 10.0, 20.0), box_(5.0, 50.0, 10.0, 10.0)];
        let result = align_boxes(&boxes, AlignMode::VCenter);
        // Union spans y 0..60; each box centers at 30.
        assert_eq!(result, vec![20.0, 25.0]);
    }

    #[test]
    fn distribute_equalizes_gaps_horizontally() {
        // Box 1 at 0 (width 10), box 2 at 100 (width 10), box 3 at 180
        // (width 20): span 0..200, total extent 40, gaps share 160/2 = 80.
        let boxes = [box_(0.0, 0.0, 10.0, 10.0), box_(100.0, 0.0, 10.0, 10.0), box_(180.0, 0.0, 20.0, 10.0)];
        let result = distribute_boxes(&boxes, DistributeAxis::Horizontal);
        assert_eq!(result, vec![0.0, 90.0, 180.0]);
    }

    #[test]
    fn distribute_keeps_extremes_anchor() {
        let boxes = [box_(0.0, 0.0, 10.0, 10.0), box_(200.0, 0.0, 10.0, 10.0)];
        // Fewer than three boxes: nothing moves.
        let result = distribute_boxes(&boxes, DistributeAxis::Horizontal);
        assert_eq!(result, vec![0.0, 200.0]);
    }

    #[test]
    fn parse_accepts_plan_mode_names() {
        assert_eq!(AlignMode::parse("left"), Some(AlignMode::Left));
        assert_eq!(AlignMode::parse("middle"), Some(AlignMode::VCenter));
        assert_eq!(AlignMode::parse("bogus"), None);
        assert_eq!(DistributeAxis::parse("x"), Some(DistributeAxis::Horizontal));
    }
}
