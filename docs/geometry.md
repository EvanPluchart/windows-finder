# Geometrie reelle du Finder (macOS 27.2, mesuree via System Events / accessibilite, mode clair)

Unites : points (ecran Retina 2940x1912 px = 1470x956 pt ; correction du Mac). Fenetre par defaut : 920x436 ; barre de titre + barre d'outils = 52.
Non lu : polices (nib : SystemUI 13/11), rayons de coins, couleurs, hauteur de la barre de chemin (masquee).

## Chrome commun
- Split view : barre laterale x0..203, separateur 1 px a x=202, contenu des x=203 (largeur 717).
- Barre d'outils 920x52 : feux tricolores 16x16 (x=18, 41, 64 ; y=18) ; Precedent/Suivant 81x52 a x=207 (2 segments 36x36) ;
  groupe de vues 162x38 a x=508 (4 segments 40x36 : icones, liste, colonnes, galerie) ; menu Grouper 47 a x=702 ;
  Partager 36 ; Tags 38 ; Action 37 ; Recherche 38x38 a x=875.
- Barre du bas (statut) : 717x28 ; compteur d'elements a gauche ; en vue icones, curseur de taille 82x16 a droite (~80 px du bord).
- Barre laterale : lignes de 32, entetes de section (Favoris, Emplacements) ~20 ; icone a x+16, libelle a x+45, bouton ejecter 14x11.

## Vues
- Liste : zone 717x384 ; entete 28 ; lignes tous les 20 (champ 18) ; triangle de depliage 15x22 a x+11 ;
  colonnes par defaut : Nom 298, Date de modification 181, Taille 97, Type 121.
- Icones : marge de section 10 ; icone 64x64 ; cellule 112x112 ; 6 colonnes dans 672 px.
- Colonnes : colonnes de 245 (+1 px separateur) ; lignes 206x23 (pas 22), retrait gauche 16 ; colonne d'apercu en derniere.
- Galerie : apercu a gauche 467 de large (image 403x261, marge 32) ; bande de vignettes 54 de haut (48x48, pas 59, premiere a x+8) ;
  inspecteur a droite 250 ; bouton « Plus... » 42x41 ; champ de tags 230x33.
