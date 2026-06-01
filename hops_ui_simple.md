# Hops Windows Desktop UI Design

## Overview

Hops uses a neo-brutalist interface style built around hard edges, visible borders, strong contrast, and offset shadows. with sidebar navigation.

---

Core rules:

- Use hard edges.
- Avoid rounded corners.
- Prefer strong borders over subtle dividers.
- Use hard offset shadows instead of soft blur shadows.
- Keep typography compact, uppercase-heavy, and utilitarian.
- Use orange as the primary accent for actions, selected states, and section labels.
- Support both light and dark themes through semantic design tokens.
- Default cards and buttons for shadow `4px 4px 0 var(--h-shadow)`
- Make interactive elements feel physical and pressable.
- Use icons when possible, use react icons

## Colors

### Light Theme (Default)

- **Background**: Ivory Cream `#FDF6E3`
- **Cards**: White `#FFFFFF`
- **Text**: Gunmetal `#233038`
- **Border**: Light Silver `#D3DBDD`
- **Accent**: Orange `#FF5B04`
- **Secondary**: Sand Yellow `#F4D47C`

### Dark Theme

- **Background**: Gunmetal `#233038`
- **Cards**: `#2a3840`
- **Text**: Ivory Cream `#FDF6E3`
- **Border**: Light Silver `#D3DBDD`
- **Accent**: Orange `#FF5B04`
- **Secondary**: Sand Yellow `#F4D47C`

---

## Layout

**Sidebar**

- Be able to expand and compact with Keyboard shortcuts (CTRL+B)
- Always **Midnight Green**: `#075056` RGB(7, 80, 86)
- Uppercase labels, Ivory Cream text
- Active item: Orange left border (3px)
- Sharp corners
- Clear distinction between selected and unselected tabs
- Hover state: Background slightly lighter (use Light Silver as border/highlight)
- Active item: Orange left border accent (3-4px thick)

**Main Content** - Cards with 1px border, full width

---

## Components

### Buttons

Buttons should feel like small physical cards that can be pressed.

- Hover should make the button feel slightly lifted.
- Pressed should make the button feel pushed down.
- Disabled buttons should keep their shape but lose emphasis.
- Avoid icon-only buttons unless the action is obvious.

### Inputs / Textareas

- Border: 1px Light Silver
- Focus: 2px Orange border
- Padding: 8-10px
- Monospaced for URLs/patterns

### Toggles

Toggles are rectangular, not rounded. They use a hard shadow and a square knob. A toggle should clearly show whether it is active or inactive. The active state should use the accent colour. The inactive state should remain visible and bordered.

- Off: Light Silver border
- On: Orange background
- Size: 18px

### Dropdowns

- Border: 1px Light Silver
- Focus: Orange border
- List hover: tinted background

### Cards

Cards are the main container pattern. They should feel like blocks placed on top of the page.
Use cards to group related controls, settings, lists, previews, or small workflows. A card should usually contain one clear purpose.
Cards should have strong borders and hard shadows. The shadow is part of the identity, not just decoration.
Keep card content structured and sparse. Do not overload a card with too many unrelated actions.

## Radio Buttons

Radio buttons are for choosing one option from a visible set.

Use radio groups when the available choices are few and benefit from being seen together. Examples include match pattern, exact URL, regex, light mode, or dark mode.

Radio buttons should feel tactile and aligned with the brutalist style. The selected state should be obvious and use the accent colour.

Radio guidance:

- Use when only one option can be selected.
- Keep labels concise.
- Stack radio options vertically when clarity matters.
- Use horizontal radio groups only for very short labels.
- Prefer radio buttons over dropdowns when there are three or fewer important options.

## Dropdowns

Dropdowns are for compact selection from a list.

Use dropdowns when the user needs to choose one option from several options, especially when showing all options upfront would take too much space.

Dropdowns should follow the same strong component language as buttons and cards: clear border, solid background, and direct typography.

The selected value should be easy to read. Placeholder text should be descriptive and not vague.

Dropdown guidance:

- Use for browser selection, route type, category, status, or sort order.
- Keep option labels short.
- Avoid nesting dropdowns inside dropdowns.
- Do not use dropdowns for two-option choices; use radio or toggle instead.
- Open menus should feel attached to the trigger, not like a separate floating panel.

---

## Typography

- **Headings**: Bold 16-18px
- **Body**: Regular 13-14px
- **Labels**: 12px, uppercase
- **Monospaced**: URLs, patterns, code

---

## Tabs

**Settings**

- Stacked settings with descriptions
- Toggle switches
- Dropdown for browser selection
- Button group at bottom

**Browsers**

- List of all browsers with each information
- Edit/Delete icons
- Add button (that opens a modal on top)

**Rules**

- List of all rules with each information
- Edit/Delete icons
- Add button (that opens a modal on top)

**Route Tester**

- URL input (monospaced, full width)
- Buttons: Preview | Route and Open
- Results panel with decision details

---

## Fonts

- Primary: "Segoe UI", Inter, sans-serif
- Monospaced: "Courier New", monospace
