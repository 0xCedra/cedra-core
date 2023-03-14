import { AnsClient } from "../../plugins/ans_client";
import { Provider } from "../../providers";
import { Network } from "../../utils";

export const ANS_OWNER_ADDRESS = "0xdc710fee87bd16028864920d50a5e444560fcbf207850f1a68cea2d606825c7c";
const ACCOUNT_ADDRESS = "0x54fac6e5d52953c75e749a8ad260bc450cad0b8ed2f06c1e98707879e13956d1";

test("fails to create a new ANS class instance", () => {
  const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
  expect(() => new AnsClient(provider)).toThrow("Please provide a valid contract address");
});

test("creates a new ANS class instance", () => {
  const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
  const ans_client = new AnsClient(provider, ANS_OWNER_ADDRESS);
  expect(ans_client).toHaveProperty("contractAddress");
});

test("sets the contract address to be the provided one", () => {
  const provider = new Provider({ fullnodeUrl: "full-node-url", indexerUrl: "indexer-url" });
  const ans_client = new AnsClient(provider, ANS_OWNER_ADDRESS);
  expect(ans_client.contractAddress).toEqual(ANS_OWNER_ADDRESS);
});

test("sets the contract address to be the matching node url", () => {
  const provider = new Provider(Network.TESTNET);
  const ans_client = new AnsClient(provider);
  expect(ans_client.contractAddress).toEqual("0x5f8fd2347449685cf41d4db97926ec3a096eaf381332be4f1318ad4d16a8497c");
});

test("get name by address", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const name = await ans.getNamebyAddress(ACCOUNT_ADDRESS);
  expect(name).toEqual("adapter");
});

test("get address by name", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const address = await ans.getAddressByName("adapter");
  expect(address).toEqual(ACCOUNT_ADDRESS);
});

test("get address by name with .apt", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const address = await ans.getAddressByName("adapter.apt");
  expect(address).toEqual(ACCOUNT_ADDRESS);
});

test("get address by subdomain_name", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const address = await ans.getAddressByName("wallet.adapter");
  expect(address).toBeNull;
});

test("get address by subdomain_name with .apt", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const address = await ans.getAddressByName("wallet.adapter.apt");
  expect(address).toBeNull;
});

test("returns null for an invalid domain", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const address = await ans.getAddressByName("adapter-");
  expect(address).toBeNull;
});

test("returns null for an invalid subdomain", async () => {
  const provider = new Provider(Network.TESTNET);
  const ans = new AnsClient(provider);

  const address = await ans.getAddressByName("wallet.adapter.apt-");
  expect(address).toBeNull;
});
