# Rust-Os

esse código é um sistema embarcado que eu tinha criado antigamente, usei o qemu para rodar ele, tenho dois códigos dele, espero que o que deixe aqui seja o que está funcionando :)

o principal problema foi criar o alocador de memória, sinceramente não queria utilizar algum já existente e inventei o meu próprio (mesmo que não será bom quanto os já difundidos em sistamas por ai como o slub allocator por exemplo).

ele apenas funciona em uma tela preta onde você pode digitar, eu meio que fui me basendo no blog do phil: https://os.phil-opp.com/

eu não tenho muito conhecimento para fazer um por conta própria ainda e não tenho conhecimento suficiente para continuar o projeto, atualmente estou estudando para isso, então esperem novidades no futuro!

eu deixei propositalmente sem alguns arquivos como o target e cargo.lock.

para quem quiser testá-lo o llvm target era "x86_64-unknown-none". Você precisa rodar rust embarcado, ou seja, com no-std e etc..
veja o blog do phil que você entenderá melhor e até conseguirá executar meu projeto.

# Este projeto receberá atualizações no futuro.
