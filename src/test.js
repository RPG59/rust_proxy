
const DRIVER = {
    async execute() {

    }
}

class Connection  {
    hasReady = Promise.resolve();

    async executeQuery(data) {
        this.hasReady = new Promise(async (res) => {
            const response = await DRIVER.execute(data.query)
            data.res(response);
            res(this);
        });
    }

    async ready() {
        return this.hasReady;
    }
}

class DB {
    connections = [];
    queue = [];
    isRunning = false;

    constructor() {
        this.connections = new Array(10).fill(0).map(() => new Connection());
    }

    async exec(query) {
        return new Promise(res => {
            this.queue.push({query, res})

            if(!this.isRunning) {
                this.isRunning = true;
                this.run();
            }
        });
    }

    async run() {
        while(this.queue.length) {
            const data = this.queue.pop();
            const connection = await Promise.any(this.connections.map(x => x.ready()))

            connection.executeQuery(data);
        }

        this.isRunning = false;
    }



}



async function main() {
    const db = new DB();
    const connection = await db.exec("SELECT * FROM users;");
}
