import { Values } from '../types';
import Wizard from '../assets/wizard.png';
import { FormikHelpers } from 'formik';
import { useMaginkContract } from './useMaginkContract';
import { NFTStorage } from 'nft.storage';
import { NFT_STORAGE_KEY } from '../const';
import CryptoJS from 'crypto-js';
import { useWallet } from 'useink';

async function fileFromPath(filePath: string) {
  const response = await fetch(Wizard);
  let data = await response.blob();

  let metadata = {
    type: 'image/png',
  };
  let file = new File([data], 'wizard.png', metadata);
  return file;
  // const content = await fs.promises.readFile(filePath);
  // const type = mime.getType(filePath)!;
  // return new File([content], path.basename(filePath), { type });
}

export const useSubmitHandler = () => {
  const { account } = useWallet();
  const { claim, mintWizard, getBadgesFor } = useMaginkContract();

  const createNFTMetadata = async () => {
    const nftstorage = new NFTStorage({ token: NFT_STORAGE_KEY });

    const image = await fileFromPath('../assets/wizard.png');
    console.log({ image });

    const result = await nftstorage.store({
      image,
      name: `${account?.address}`,
      description: 'Wizard NFT reward for completing Magink challenges',
    });
    return result;
  };

  return async (values: Values, { setSubmitting }: FormikHelpers<Values>) => {
    console.log('send claim Tx');
    const badgesEarned = await getBadgesFor?.send([account?.address], { defaultCaller: true });

    if (badgesEarned?.ok && badgesEarned.value.decoded >= 9) {
      console.log('##### badges earned count', badgesEarned?.ok && badgesEarned.value.decoded);
      console.log('send mint wizard transaction');
      const result = await createNFTMetadata();
      console.log({ metadata_result: result });
      const hash = CryptoJS.SHA256(result.ipnft).toString();
      const mintArgs = [hash];
      mintWizard?.signAndSend(mintArgs, undefined, (_result, _api, error) => {
        if (error) {
          console.error(JSON.stringify(error));
          setSubmitting(false);
        }

        setSubmitting(false);
      });
    } else {
      console.log('send claim Tx');
      const claimArgs = undefined;
      const options = undefined;
      claim?.signAndSend(claimArgs, options, (_result, _api, error) => {
        if (error) {
          console.error(JSON.stringify(error));
          setSubmitting(false);
        }

        // if (!result?.status.isInBlock) return;

        setSubmitting(false);
      });
    }
  };
};
