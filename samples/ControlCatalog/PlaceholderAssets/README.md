# Project artwork

The artwork of this project that stands in for the assets of `Assets/` that carry the brand of the
upstream project. With the feature `placeholder-branding` of this crate, which is on by default (the
desktop host and the browser host `samples/ControlCatalog.Browser` both show it), the build script
embeds each file of this directory in place of the asset of the same name under `Assets/`, and
replaces the path data of each `StreamGeometry` resource whose key names a file of `StreamGeometry/`
with the content of that file. Every file here must replace an asset or a resource; the build fails
otherwise. `Assets/` itself stays an unchanged copy of the upstream sample.

| File | Replaces | Content |
|---|---|---|
| `icon-32.png` | the application icon (32 pixels) | the mark |
| `icon.ico` | the application icon | the mark at 16, 32, 48 and 256 pixels |
| `banner-logo.png` | the word mark on the banner of the home page | the mark and the name of the project |
| `banner-bg.png` | the background of the banner of the home page | steel blue with the glow of heated iron |
| `logo1.png` to `logo4.png` | the product artwork floating on the banner | a crystal, a plate, a ring and a spark |
| `StreamGeometry/AppIcon.txt` | the resource `AppIcon` of `App.xaml`, the logo at the top of the navigation pane | the mark as path data (48 units square, even-odd fill) |

The mark is `samples/ControlCatalog.Browser/wwwroot/favicon.svg`: the letter F and a spark on a
rounded square, original artwork. The files are drawn by
`scripts/browser/render-placeholder-assets.mjs`; run it again after changing the artwork and commit
the result.
