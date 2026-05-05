# Theme Color Reference

Set theme colors globally, then override per-widget with `t:rule()`.

## Global color roles

| Role | What it colors |
|------|---------------|
| `primary` | Brand accent; interactive control fill |
| `secondary` | Alternate accent for secondary emphasis |
| `background` | Page-level background |
| `surface` | Elevated surfaces above background |
| `text` | Primary text for readability |
| `text_muted` | Subdued text for secondary or disabled content |
| `accent` | Emphasized accent for active or focused elements |
| `border` | Outline color for widgets and containers |
| `hover` | Hover or pressed overlay |
| `danger` | Error, destructive action, or critical warning |
| `selection` | Text-selection highlight background |
| `highlight` | Transient highlight for active or matching elements |
| `placeholder` | Fill for placeholder or ghost content |
| `overlay` | Scrim or backdrop for overlays and modals |
| `panel` | Large surface for panels and sidebars |
| `panel_text` | Text rendered on panel surfaces |

## Per-widget rule fields

These are the fields you can set in `t:rule(Widget.X, { ... })`.

### Button

| Field | What it colors |
|-------|---------------|
| `primary` | Button fill |
| `text` | Label text |
| `text_muted` | Blocked state text |
| `border` | Outline |
| `hover` | Hover overlay |

### Slider

| Field | What it colors |
|-------|---------------|
| `primary` | Handle fill |
| `background` | Track fill |
| `surface` | Track gutter |
| `border` | Handle outline |
| `hover` | Handle when dragging |

### Panel

| Field | What it colors |
|-------|---------------|
| `background` | Panel surface |
| `border` | Panel outline |

### Label

| Field | What it colors |
|-------|---------------|
| `text` | Label text |

