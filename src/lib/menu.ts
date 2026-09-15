export interface MenuItem {
  label: string;
  /** Raccourci affiché à droite, purement indicatif. */
  keys?: string;
  /** Coche à gauche, pour les entrées qui reflètent un état. */
  checked?: boolean;
  /** Désactivé quand l'action n'a rien à faire (ex. « fermer les autres »
      alors qu'il n'y a qu'un onglet). */
  disabled?: boolean;
  separatorBefore?: boolean;
  run: () => void;
}
