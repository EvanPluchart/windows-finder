# Spec Finder -> Windows (Rust + GPUI)

Source: analyse statique de Finder.app (macOS 27.2, build 1865.1.10) faite par la session Mac, lecture seule.
Le CLI REA n'a pas encore ete execute (attente d'approbation de l'utilisateur sur le Mac).

## Faits observes (binaire)
- Bundle id `com.apple.finder`, binaire universel arm64e, 126 load commands.
- Schemes URL: afp, file, ftp, smb, nwnode. Types: Folder, Volume, Saved Search (.savedSearch), Burn Folder (.fpbf), Clippings.
- Frameworks lies: QuickLook, QuickLookUI, QuickLookThumbnailing, CoreSpotlight, IconServices, Sharing(UI), CloudSharing(UI), DesktopServicesUI.
- Non lies: FinderKit, SidebarServices, TagKit (les tags sont des classes `FITag*` internes).
- Ressources: Assets.car (128 assets), nibs BrowserWindow / GalleryPreviewView / ProgressWindow, `default_smart.plist`, `GroupByKind.plist`, `InfoDisplayOptions.plist`.

## Modules (classes Finder -> module Rust)
| Zone | Classes Finder | Module |
|---|---|---|
| Fenetre / onglets | TBrowserWindowController, NSTabBarDelegate | `window`, `tabs` |
| Vue icones | TIconView, TDesktopIconView | `views::icon` |
| Vue liste | TListView, TListViewSettings | `views::list` |
| Vue colonnes + volet apercu | TColumnView, TColumnPreviewController | `views::column` |
| Vue galerie | TGalleryCollectionView, TGalleryPreviewViewController | `views::gallery` |
| Barre laterale | TSidebarViewController, TSidebarItemCell | `sidebar` |
| Barre d'outils | Back/Forward, Arrangement, Path, Location, QuickLook, Action | `toolbar` |
| Barre de chemin / statut | TPathBarController, TStatusBarController | `pathbar`, `statusbar` |
| Apercu Espace | TQLPreviewWindowController, TQLPreviewViewZoomController | `quicklook` |
| Recherche / smart folders | TSearchField, TScopeBarRadioGroupView, `*_smart.plist` | `search` |
| Tags | FITagEditorView, TICloudTagSyncAdapter | `tags` |
| Get Info | TInfoWindow* | `info` |
| Spring-loading, drag & drop | TSpringLoadingDestinationDelegate, TDraggingSource | `dnd` |

## Fonctionnalites cibles (par priorite)
1. P0: fenetre, barre laterale, vue liste, navigation (retour/avant/parent), chemin, barre de statut, selection clavier, indexation rapide.
2. P0: Quick Look (Espace) : fenetre flottante, zoom, fleches pour changer de fichier, images/texte/PDF/video.
3. P1: vues icones, colonnes (avec apercu), galerie ; onglets ; recherche + dossiers intelligents ; tri/groupement (GroupByKind).
4. P2: tags, Get Info, spring-loading, glisser-deposer, partage, volets d'options d'affichage.

## A valider (pas encore observe)
- Comportement exact de positionnement/animation de Quick Look (QuickLookUI est dans le cache dyld, non analyse).
- Valeurs de design (couleurs, espacements, tailles d'icones) : a tirer de Assets.car et de captures.
