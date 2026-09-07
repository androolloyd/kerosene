use super::*;

#[test]
fn pane_dividers_hide_lines_without_changing_surfaces_or_geometry() {
    for theme in [Theme::Dark, Theme::Light] {
        let visible = pane_content_style(&theme, 12.0, true);
        let hidden = pane_content_style(&theme, 12.0, false);
        assert!(visible.border.color.a > 0.0);
        assert_eq!(hidden.border.color, Color::TRANSPARENT);
        assert_eq!(hidden.background, visible.background);
        assert_eq!(hidden.border.width, visible.border.width);
        assert_eq!(hidden.border.radius, visible.border.radius);

        let visible_title = pane_title_bar_style(&theme, 12.0, true);
        let hidden_title = pane_title_bar_style(&theme, 12.0, false);
        assert!(matches!(
            visible_title.background,
            Some(iced::Background::Gradient(_))
        ));
        assert_eq!(
            hidden_title.background,
            Some(theme.extended_palette().background.strong.color.into())
        );
        assert_eq!(hidden_title.border, visible_title.border);

        let visible_grid = pane_grid_style(&theme, 12.0, 8.0, true);
        let hidden_grid = pane_grid_style(&theme, 12.0, 8.0, false);
        assert_eq!(visible_grid.hovered_split.color, theme.palette().primary);
        assert_eq!(hidden_grid.hovered_split.color, Color::TRANSPARENT);
        assert_eq!(hidden_grid.picked_split.color, Color::TRANSPARENT);
        assert_eq!(
            hidden_grid.hovered_split.width,
            visible_grid.hovered_split.width
        );
        assert_eq!(
            hidden_grid.picked_split.width,
            visible_grid.picked_split.width
        );
        // Drag-and-drop placement feedback remains visible.
        assert_eq!(
            hidden_grid.hovered_region.background,
            visible_grid.hovered_region.background
        );
    }
}
