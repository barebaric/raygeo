import cairo

try:
    from gi.repository import Pango, PangoCairo
except ImportError:
    Pango = None
    PangoCairo = None


def create_text_layout(
    ctx: cairo.Context,
    text: str,
    font_size: float = 12,
    *,
    family: str = "Sans",
    weight: "Pango.Weight | None" = None,
    style: "Pango.Style | None" = None,
    absolute_size: bool = True,
) -> "Pango.Layout":
    if Pango is None or PangoCairo is None:
        raise RuntimeError(
            'create_text_layout requires PyGObject (gi)'
        )
    if weight is None:
        weight = Pango.Weight.NORMAL
    if style is None:
        style = Pango.Style.NORMAL
    layout = PangoCairo.create_layout(ctx)
    font = Pango.FontDescription()
    font.set_family(family)
    font.set_weight(weight)
    font.set_style(style)
    if absolute_size:
        font.set_absolute_size(font_size * Pango.SCALE)
    else:
        font.set_size(round(font_size * Pango.SCALE))
    layout.set_font_description(font)
    layout.set_text(text, -1)
    return layout


def draw_centered_text(
    ctx: cairo.Context,
    text: str,
    width: float,
    height: float,
    font_size: float = 12,
    *,
    family: str = "Sans",
    weight: "Pango.Weight | None" = None,
    style: "Pango.Style | None" = None,
    absolute_size: bool = True,
) -> "Pango.Layout":
    layout = create_text_layout(
        ctx,
        text,
        font_size,
        family=family,
        weight=weight,
        style=style,
        absolute_size=absolute_size,
    )
    extents, _ = layout.get_pixel_extents()
    ctx.move_to(
        (width - extents.width) / 2 - extents.x,
        (height - extents.height) / 2 - extents.y,
    )
    PangoCairo.show_layout(ctx, layout)
    return layout
