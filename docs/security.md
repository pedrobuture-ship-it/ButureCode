# Segurança

Nenhuma ferramenta será instalada automaticamente no SO. Instalações, operações destrutivas, execução de comandos do workspace e envio de contexto exigirão fluxos explícitos nos marcos futuros.

Não registrar secrets nem versionar `.env`. Credenciais futuras usarão armazenamento seguro do SO quando disponível. SSH respeitará known_hosts. Extensões deverão ser isoladas e IA será opcional, sem transmissão automática.

Este bootstrap não executa comandos de projetos, não acessa rede em runtime e não possui telemetria. Não transformar nomes de arquivos em comandos shell. `unsafe_code` é proibido no código autoral; dependências serão avaliadas separadamente.
