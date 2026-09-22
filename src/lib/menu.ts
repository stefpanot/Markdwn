export interface MenuItem {
  label: string;
  /** Raccourci affiché à droite, purement indicatif. */
  keys?: string;
  /** Coche à gauche, pour les entrées qui reflètent un état. */
  checked?: boolean;
  /** Désactivé quand l'action n'a rien à faire (ex. « fermer les autres »
      alors qu'il n'y a qu'un onglet). */
  disabled?: boolean;
  /** Entête de section (ex. « Édition ») : affichée quand l'entrée précédente
      appartient à un autre groupe. Regroupe les entrées façon menu classique,
      un seul dropdown conservé. */
  group?: string;
  separatorBefore?: boolean;
  run: () => void;
}
