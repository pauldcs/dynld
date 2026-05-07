# dynld: macOS/iOS Dynamic Linker

Disclaimer: ce projet n'a pas d'utilité pratique, le kernel XNU ne vous laisse pas la liberté d'utiliser le linker dynamique de votre choix pour éxecuter vos programmes. Tous linker dynamique déclaré à travers un `LC_LOAD_DYLINKER` est vérifié par le kernel, et doit correspondre à `/usr/lib/dyld`. Je vous déconseille d'avoir la curiosité de voir ce qu'il se passe si on le remplace par un autre, vous allez tout pêter instantanément de manière catastrophique, sauf si vous êtes un génie.
La résponsabilité que `dyld` porte est sincèrement réspectable. Aucun programme, à part `dyld` ne peut s'éxecuter sans `dyld`.
Ce programme connait tous les secrets des binaires depuis la nuit des temps. Il sait avec précision comment gérer les Mach-O dans toutes ses formes. Jamais ce projet n'atteindra un tel niveau malheureusement. D'autant plus qu'il est difficile de se documenter sur ce sujet, je ne suis personellement pas parvenu à trouver d'articles(particulièrement sur l'internet francais) sur ce que nécessite l'implémentation d'un linker dynamique sous les plateforme Apple. Ce projet vise à aider ceux qui souhaitent executer des programmes sans `dlopen`, `dlsym`, `execve` etc.

### Implementer un linker dynamique

Tout d'abord, ce projet est en Rust `no_std`. Géneralement les apps `no_std` sont moins `no_std` qu'ici. Les Mach-O `no_std` béneficient généralement tout de même d'un linkage dynamique. Certains pointeurs que ces binaires utilisent, ne sont en fait que des offsets qui sont transformés en pointeurs par `dyld`, au moment du linkage dynamique. Cela se confirme avec la commande `dyld_info <Mach-O no_std> -fixups`:
```
-fixups:
    segment         section          address             type   target
    __DATA_CONST    __const          0x000D0000         rebase  0x000CB58C
    __DATA_CONST    __const          0x000D0018         rebase  0x000CB58C
    __DATA_CONST    __const          0x000D0030         rebase  0x000CB58C
    __DATA_CONST    __const          0x000D0048         rebase  0x000CB58C
    __DATA_CONST    __const          0x000D0060         rebase  0x000CB598
...
```
Le terme `rebase` fait référence aux endroits du programme qui doivent être `rebase`, c'est à dire qu'ils doivent être dynamiquement modifiés afin de contenir l'addresse réelle en mémoire de ce qui est pointé. Ces pointeurs peuvent par exemple pointer sur des vtables et ce genre de choses. Lorsque le linker dynamique s'éxecute, il doit pouvoir s'auto linker. D'autant plus que depuis macOS 12 et iOS 15, Apple utilise les `chained fixups`, qui est le format d'encodage des informations ci-dessus. Ce format à été inventé pour remplacer les `LC_DYLD_INFO(_ONLY)`, estimé trop lent et prenant trop de place dans les binaires. Ce format est relativement nouveau, mais il existe tout de même des exemples d'implémentation de parseurs, notamment un plugin Binary Ninja, qui est opensource (et en Python).

Le linker dynamique est un type particulier de Mach-O. Son exécution est complètement differente de celle d'un binaire normal. C'est un programme destiné à être exécuté, mais pas via `execve`.

Sur les plateformes Apple, c'est en pratique le seul programme qui s'exécute de manière statique. Tout le reste doit linker au minimum à `libSystem.dylib`. Cela tient au fait que l'ABI des appels système est instable. Il existe d'autres raisons de ne pas se passer de `libSystem`, mais si vous écrivez le linker dynamique lui-même, vous n'avez pas vraiment le choix. Beaucoup de choses qui fonctionnent normalement cessent de fonctionner, souvent de manière difficile à debugger. La plupart de ces problèmes relèvent de "détails d'implémentation" non documentés. Il existe peu de ressources en ligne expliquant comment s'y prendre concrètement. La meilleure référence reste le dyld d'Apple, qui est open source: https://github.com/apple-oss-distributions/dyld. Je mettrai également des liens vers des articles et d'autres ressources qui m'ont aidé en chemin.

Ce projet n'est qu'une implémentation de base d'un linker dynamique Apple. Il sera moins bon que dyld sur tous les plans. L'objectif est de fournir un point de départ utile à quiconque en aurait besoin.
Il est également à noter que l'invocation de ce linker dynamique est légerement différente de celle de `dyld`, pour `dynld`, en plus de fournir l'addresse de l'executable à éxecuter, le kernel doit fournir la taille de l'image.

### Tester

Un outil situé dans `tools/launcher` reproduit la manière dont le kernel invoquerait le linker dynamique lors d'un `execve`. Il charge l'exécutable, charge le linker dynamique, prépare la stack (argc/argv, etc.), effectue une analyse basique du linker pour mapper ses segments, localise le point d'entrée et jump dessus. XNU fait beaucoup d'autres choses en plus de cela, principalement sans rapport avec le linker dynamique, mais c'est en gros ce qu'`execve` est concu pour faire.
```sh
$ cd tools/laucher
$ make
...
$ file ../../target/aarch64-apple-darwin/debug/dynld
Mach-O 64-bit dynamic linker arm64
$ ./run /bin/ls ../../target/aarch64-apple-darwin/release/dynld
```
Vous pouvez également compiler avec `make g` afin de béneficier d'outillage de debug, tel q'`fsanitize` lorsque le linker dynamique ne marche pas correctement. Debugger `dynld` n'est pas une mince affaire, mais l'éxcellent `lldb` reste très utile. Il est difficile de provoquer des breakpoints, que ce soit dans le linker dynamique ou dans le programme qu'il execute. N'hésitez pas à recompiler les programmes testés, et d'y ajouter manuellement des `__builtin_debugtrap()` aux endroits que vous voulez inspecter, `fsanitize` vous aidera à savoir où se situe les problèmes, et vers où regarder.
Le dossier `tests/binaries` contient plusieurs programmes de test. Il sont tous concus pour écrire `Hello, World!` sur stdout de manières différentes, en testant différentes choses. À l'heure ou j'écris, environ 15 des ~20 binaires fonctionnent. Vous pouvez également jeter un oeuil à la CI en cliquant sur le X rouge à côté du dernier commit pour voir ce qui marche et ce qui ne marche pas sans avoir à réexecuter les tests sur votre machine. Une chose majeur qui manque, est le support des programmes Objective-C (donc pas de support des programmes Swift non plus). J'éspère ajouter cette feature au plus vite.

### Liens
Voici des liens intéressants sur ce sujet
- https://en.wikipedia.org/wiki/Mach-O
- https://karol-mazurek.medium.com/dyld-do-you-like-death-i-8199faad040e
- https://www.mikeash.com/pyblog/friday-qa-2012-11-09-dyld-dynamic-linking-on-os-x.html
- https://www.mikeash.com/pyblog/friday-qa-2012-11-30-lets-build-a-mach-o-executable.html
- https://embeddedartistry.com/blog/2019/05/20/exploring-startup-implementations-os-x/
- https://jano.dev/apple/mach-o/2024/11/27/Launching-A-Binary.html
- https://karol-mazurek.medium.com/snake-apple-i-mach-o-a8eda4b87263
- https://www.apriorit.com/dev-blog/225-dynamic-linking-mach-o
- https://github.com/qyang-nj/llios/blob/main/dynamic_linking/chained_fixups.md
- https://www.emergetools.com/blog/posts/iOS15LaunchTime
- https://gist.github.com/BertalanD/6a11e6e658be7d834870b03fb3e8af7b
- https://github.com/xpcmdshell/bn-chained-fixups
- https://github.com/apple-oss-distributions/dyld/tree/main/doc
- https://github.com/apple-oss-distributions/dyld/blob/main/dyld/dyldMain.cpp
- https://github.com/opensource-apple/dyld/blob/master/src/dyldInitialization.cpp
- https://github.com/darlinghq/darling
- https://blog.darlinghq.org/2018/07/mach-o-linking-and-loading-tricks.html