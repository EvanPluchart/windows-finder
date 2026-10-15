# Raccourcis du Finder (source : MenuBar.nib, via REA decode-interface-builder, macOS 27.2)

Limites : ordre = id d'objet, pas l'ordre d'affichage ; les variantes alternates (touche Option) sont melangees.
Doute : un second "Rotate Left" a ⌘L et ⌘R sont probablement des items Quick Look caches.

Mapping propose Windows : ⌘ -> Ctrl, ⌥ -> Alt, ⌃ -> Ctrl+Win (a affiner), ⇧ -> Shift.
Conflits Windows connus : Ctrl+Y (Redo natif), Alt+lettres (menus), Ctrl+W/T (onglets, OK), Ctrl+I.

## Finder (-> menu appli)
Preferences ⌘, | Vider la corbeille ⇧⌘Del (⌥ = sans confirmation) | Quitter ⌘Q

## Fichier
Nouvelle fenetre ⌘N | Nouveau dossier ⇧⌘N | Dossier avec selection ⌃⌘N | Nouvel onglet ⌘T | Ouvrir ⌘O |
Ouvrir dans nouvelle fenetre ⌃⌘O | Fermer fenetre ⌘W | Lire les informations ⌘I | Inspecteur ⌥⌘I |
Dupliquer ⌘D | Creer un alias ⌃⌘A | Afficher l'original ⌘R | Ajouter a la barre laterale ⌃⌘T |
**Aperçu rapide (Quick Look) ⌘Y** (et Espace) | Diaporama ⌥⌘Y | Mettre a la corbeille ⌘Del |
Supprimer immediatement ⌥⌘Del | Ejecter ⌘E | Imprimer ⌘P
Sans raccourci : Compresser, Renommer, Tags, Partager, Ouvrir avec, Personnaliser le dossier.

## Edition
Annuler ⌘Z | Retablir ⇧⌘Z | Couper ⌘X | Copier ⌘C | Coller ⌘V | Deplacer ici ⌥⌘V | Tout selectionner ⌘A |
Deselectionner ⌥⌘A | Rechercher ⌘F | Focus champ recherche ⌥⌘F | Copier le chemin ⌥⌘C

## Presentation
Icones ⌘1 | Liste ⌘2 | Colonnes ⌘3 | Galerie ⌘4 | Options d'affichage ⌘J |
Barre laterale ⌃⌘S | Barre d'outils ⌥⌘T | Barre de chemin ⌥⌘P | Barre d'etat ⌘/ | Apercu ⇧⌘P |
Barre d'onglets ⇧⌘T | Tous les onglets ⇧⌘\ | Groupes ⌃⌘0 | Taille icones ⌘+ / ⌘- | Plein ecran ⌃⌘F
Trier/grouper : Nom ⌥⌘1, Type ⌥⌘2, Derniere ouverture ⌥⌘3, Date d'ajout ⌥⌘4, Date de modif ⌥⌘5, Taille ⌥⌘6, Tags ⌥⌘7

## Aller
Precedent ⌘[ | Suivant ⌘] | Dossier parent ⌘Haut | Ouvrir la selection ⌘Bas | Recents ⇧⌘F | Documents ⇧⌘O |
Bureau ⇧⌘D | Telechargements ⌥⌘L | Accueil ⇧⌘H | iCloud Drive ⇧⌘I | Partages ⇧⌘S | AirDrop ⇧⌘R | Reseau ⇧⌘K |
Ordinateur ⇧⌘C | Applications ⇧⌘A | Utilitaires ⇧⌘U | Aller au dossier ⇧⌘G | Se connecter au serveur ⌘K

## Fenetre
Reduire ⌘M | Cycle fenetres ⌘` | Fenetre de progression

## Resultats a retenir pour le clone
- Quick Look = ⌘Y ET Espace (Espace est gere hors menu, par la vue).
- 4 vues numerotees 1 a 4 ; tri par ⌥⌘1..7.

## Raccourcis implementes dans l'application Windows (etat actuel)
Navigation : fleches ; Ctrl+O / Ctrl+Bas ouvrent ; Ctrl+Haut / Alt+Haut dossier parent ; Alt+Gauche/Droite ou Ctrl+[ / Ctrl+] historique.
Selection : clic, Ctrl+clic (basculer), Maj+clic (plage), Maj+fleches, Ctrl+A.
Fichiers : Entree ou F2 renomme (comme le Finder) ; Ctrl+Maj+N nouveau dossier ; Ctrl+C / Ctrl+X / Ctrl+V ; Ctrl+D duplique ;
           Suppr ou Ctrl+Retour arriere = corbeille Windows (reversible, jamais de suppression definitive).
Vues : Ctrl+1..4 (AZERTY : Ctrl+& e " ') ; Ctrl+Maj+. fichiers caches ; F5 / Ctrl+R actualise.
Autres : Espace ou Ctrl+Y apercu ; Ctrl+Maj+G aller au dossier ; Ctrl+F recherche ; Echap ferme la couche en cours ; clic droit menu contextuel.
Editeur (apercu .txt / cellule CSV) : Ctrl+Z annule, Ctrl+Y ou Ctrl+Maj+Z retablit, Ctrl+S enregistre.
Tri : clic sur un en-tete de colonne (Nom, Date, Taille, Type), second clic = ordre inverse. Ctrl+I ou menu = Lire les informations (Echap ferme).
Glisser-deposer : glisser des elements sur un dossier (vue, barre laterale, chemin) = deplacer (Ctrl = copier) ; en vue Icones, deposer sur le fond = placement libre (clic droit > Ranger les icones pour revenir a la grille). Elements caches/systeme masques comme l'Explorateur.
Presse-papiers : Ctrl+C / Ctrl+X / Ctrl+V utilisent le presse-papiers Windows (fichiers echanges avec l'Explorateur). Souris : bouton precedent / suivant = historique.
Barre laterale : Applications (programmes du menu Demarrer, clic droit > Desinstaller avec confirmation), Ce PC, disques, Reseau et Corbeille (ouverts dans l'Explorateur). Pastille verte : survol = popover Deplacer/Redimensionner/Plein ecran.
Selection : taper le debut d'un nom selectionne l'element (1 s) ; glisser sur le fond de la vue Icones dessine un rectangle de selection.
Onglets : Ctrl+T nouveau, Ctrl+W ferme, Ctrl+Tab / Ctrl+Maj+Tab bascule, clic milieu sur un emplacement = nouvel onglet. Mode sombre : suit Windows. Copie/deplacement : boite de conflit (Arreter, Ignorer, Garder les deux, Remplacer, Appliquer a tous) et progression.
Options d'affichage (Ctrl+J) : par dossier (mode, tri, taille d'icones, espacement, texte, colonnes, dates relatives), 'Par defaut' / 'Restaurer'. Memorise dans %APPDATA%\winfinder\folders.tsv.
