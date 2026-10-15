# Dispositions des vues Finder (source : nibs decodes via REA, macOS 27.2)

Les tailles sont des valeurs de conception Interface Builder : des indications, pas de la geometrie finale
(le decodeur n'infere pas les contraintes).

## Vue liste (ListView.nib)
Polices : cellules 13 pt, en-tete 11 pt (SystemUI). En-tete 23 px. Colonnes (largeur / min / max) :
| Colonne | Largeur | Min | Max |
|---|---|---|---|
| Nom | 200 | 200 | 20000 |
| Taille | 110 | 75 | 220 |
| Date de modification | 115 | 100 | 350 |
| Date de creation | 115 | 100 | 350 |
| Date d'ajout | 115 | 100 | 350 |
| Derniere ouverture | 135 | 125 | 350 |
| Type | 100 | 60 | 500 |
| Version | 75 | 75 | 200 |
| Commentaires | 100 | 100 | 1000 |
| Tags | 70 | 70 | 400 |
| Statut iCloud | 200 | 30 | 300 |
| Partage par / Derniere modif par / Invitation | 200 | 200 | 20000 |
Colonnes visibles par defaut : a verifier (Nom, Date de modification, Taille, Type d'apres l'usage).

## Volet d'apercu des colonnes (ColumnPreview.nib), largeur 312
Pile verticale, marge interne 10 px, pas de ligne 21 px :
1. Apercu Quick Look : 292x293 (bloc 312x303).
2. En-tete (88 px) : icone 48x48, puis nom du fichier (bulle 216x41), ligne taille/type, ligne tags.
3. "Informations" (224 px) : titre + bouton "Afficher plus/moins", lignes de 20 px (Partage par, Cree, Modifie, Derniere ouverture, Version),
   metadonnees Spotlight, champ de tags (« Ajouter des tags… »).

## Barre superieure (BrowserOverlayStackView.nib), hauteur ~60, largeur 800 de reference
- Gauche (x=20, 190x24) : controle segmente des vues (47x24), menu Options d'affichage (63x24), menu Organiser par (64x24).
  Info-bulle : « Maintenir Option pour changer le tri ».
- Centre : ligne « Ou : » + menu d'emplacement 198x20 + bouton 24x20 (portee de recherche).
- Droite (x=590) : champ de recherche avec suggestions 190x24.
- Separateurs 1 px en haut et en bas.

## Portee de recherche (SearchScopeSlice.nib), 476x32
« Rechercher : » + boutons radio (« Ce Mac », « <dossier> ») + « Enregistrer » (48x20) + bouton « + » (31x20).

## Aller au dossier (GotoWindow.nib)
Fenetre sans bordure, fond translucide (NSVisualEffectView) 460x50 : champ de chemin 346x20 (« Entrez un chemin »),
texte fantome d'autocompletion, bouton fermer ; liste de suggestions qui s'ouvre sous le champ (hauteur 0 au repos, lignes 20 px).

## Couleurs (Assets.car)
Ce sont les couleurs de l'icone du Finder (visage bleu), pas du chrome de l'interface.
Bleu P3 : (0, 0.761, 1), (0.22, 0.645, 1), (0, 0.70, 1). Gris sombre : 0.192, 0.078. Blanc, noir.
=> Le style du chrome (barres, fond, selection) vient des controles systeme macOS : a prendre sur captures.
