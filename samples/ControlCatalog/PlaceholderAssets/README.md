# Placeholder artwork

Neutral stand-ins for the assets of `Assets/` that carry the brand of the upstream project. With the
feature `placeholder-branding` of this crate, which the browser host `samples/ControlCatalog.Browser`
selects for its published site, the build script embeds each file of this directory in place of the
asset of the same name under `Assets/`, and replaces the path data of each `StreamGeometry` resource
whose key names a file of `StreamGeometry/` with the content of that file. Every file here must
replace an asset or a resource; the build fails otherwise.

| File | Replaces | Content |
|---|---|---|
| `icon-32.png` | the application icon (32 pixels) | the placeholder mark |
| `icon.ico` | the application icon | the placeholder mark at 16, 32, 48 and 256 pixels |
| `banner-logo.png` | the word mark on the banner of the home page | the placeholder mark and the name of the project |
| `banner-bg.png` | the background of the banner of the home page | a neutral gradient |
| `logo1.png` to `logo4.png` | the product artwork floating on the banner | plain translucent shapes |
| `StreamGeometry/AppIcon.txt` | the resource `AppIcon` of `App.xaml`, the logo at the top of the navigation pane | the placeholder mark as path data (48 units square, even-odd fill) |

The placeholder mark is `samples/ControlCatalog.Browser/wwwroot/favicon.svg`, original artwork. The
files are drawn by `scripts/browser/render-placeholder-assets.mjs`; run it again after changing the
artwork and commit the result.
