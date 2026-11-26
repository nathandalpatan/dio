"""Build and validate DIO's fixed, purposive 500-item benchmark corpus."""

import csv
import random
from pathlib import Path


SEED = 20261125
OUTPUT = Path(__file__).resolve().parents[1] / "benchmark_words.csv"

# Each pipe-separated row is term | primary context | second context (when needed).
# These are authored usage prompts, not definitions or independent references.
GROUPS = {
    "everyday": """
apple|She packed an apple for lunch.
door|Please close the door behind you.
window|Rain tapped against the window.
table|The keys are on the table.
chair|Pull up a chair beside me.
garden|Tomatoes grow in the garden.
morning|The train arrives every morning.
evening|We walked home in the evening.
friend|A friend offered to help.
family|Her family came for dinner.
street|The market is across the street.
school|The school opens at eight.
teacher|The teacher explained the lesson.
student|Each student received a notebook.
coffee|He made coffee before work.
bread|Fresh bread cooled on the counter.
water|Drink some water after the run.
rain|The rain stopped before noon.
sunny|It was sunny all afternoon.
cloud|A dark cloud covered the sun.
quiet|The library was quiet today.
hungry|I felt hungry after swimming.
happy|She looked happy with the result.
sad|The ending made him sad.
tired|We were tired after the trip.
early|The bus came early today.
late|She arrived late for class.
quickly|He quickly crossed the road.
slowly|The queue moved slowly.
borrow|Can I borrow your umbrella?
return|Please return the book tomorrow.
repair|They will repair the broken fence.
choose|Choose a seat near the front.
carry|I can carry the heavy bag.
clean|Please clean the kitchen floor.
warm|The blanket kept her warm.
cold|The soup is already cold.
empty|The bottle was empty by noon.
full|The car park was full.
price|Check the price before buying.
receipt|Keep the receipt for a refund.
ticket|He bought a ticket at the station.
journey|The journey took three hours.
holiday|They planned a holiday in June.
neighbour|Our neighbour watered the plants.
library|I borrowed a novel from the library.
picture|She hung a picture above the desk.
message|I sent a message to my sister.
question|The question has a simple answer.
answer|Write your answer on the form.
""",
    "advanced": """
ephemeral|The flowers' ephemeral beauty faded by dusk.
ubiquitous|Smartphones are ubiquitous on the train.
pragmatic|They chose a pragmatic solution within budget.
ostensibly|The meeting was ostensibly about staffing.
nuanced|Her nuanced argument acknowledged both risks.
pedantic|His pedantic corrections slowed the discussion.
ambivalent|She felt ambivalent about moving abroad.
equivocal|The witness gave an equivocal answer.
intransigent|The committee remained intransigent during talks.
perfunctory|He gave a perfunctory apology and left.
obfuscate|The report seemed designed to obfuscate the costs.
ameliorate|New insulation should ameliorate the cold.
exacerbate|The delay could exacerbate the shortage.
juxtapose|The exhibit will juxtapose old and new photographs.
mitigate|Trees can mitigate summer heat.
inchoate|Her inchoate idea needed further work.
deleterious|The chemical had a deleterious effect on fish.
salient|The judge summarized the salient facts.
esoteric|The lecture covered esoteric legal history.
lucid|His lucid explanation helped the class.
tenuous|The proposed link remains tenuous.
ineffable|She described an ineffable sense of relief.
capricious|The capricious weather ruined our plans.
cogent|The lawyer made a cogent argument.
parsimonious|A parsimonious model needs fewer parameters.
prolific|The prolific author released another novel.
anachronistic|The rotary phone looked anachronistic in the office.
apocryphal|The apocryphal story spread through town.
contentious|The zoning proposal became contentious.
discerning|The discerning buyer noticed the flaw.
disparate|The survey combined disparate groups.
eloquent|Her eloquent speech held the room.
fastidious|He was fastidious about punctuation.
garrulous|The garrulous host talked through dinner.
idiosyncratic|Her idiosyncratic method still worked.
implacable|The two sides faced implacable opposition.
laconic|His laconic reply was just one word.
magnanimous|She was magnanimous after winning.
obdurate|The obdurate manager refused to reconsider.
perspicacious|A perspicacious reader spotted the clue.
recalcitrant|The recalcitrant witness ignored the summons.
reticent|He was reticent about his childhood.
sanguine|Despite setbacks, she remained sanguine.
scrupulous|The auditor was scrupulous with the records.
spurious|The article relied on spurious claims.
substantiate|Please substantiate the figures with receipts.
sycophantic|The sycophantic praise embarrassed her.
ubiquity|The ubiquity of cameras changed the debate.
vacuous|His vacuous response answered nothing.
vicissitude|They adapted to every vicissitude of the season.
""",
    "polysemous": """
run|I run five kilometres each morning.|The theatre will run the play through June.
set|Set the cup on the shelf.|The sun set behind the hills.
bank|She deposited the cheque at the bank.|The child sat on the river bank.
fine|He paid a parking fine.|The fabric had a fine texture.
light|Turn on the light in the hall.|This suitcase is light enough to carry.
sanction|The regulator imposed a sanction on the firm.|The board will sanction the proposed sale.
pitch|The singer held a high pitch.|They played football on the pitch.
spring|Flowers bloom in spring.|The mattress has a broken spring.
crane|A crane lifted the steel beam.|A crane stood in the marsh.
bark|The dog's bark woke us.|The bark peeled from the tree.
seal|A seal swam near the shore.|Press the lid to seal the jar.
draft|She wrote a draft of the letter.|A cold draft came through the window.
current|The river current was strong.|The current policy expires tomorrow.
case|The lawyer prepared the case.|Put the camera in its case.
scale|The fish had shiny scales.|The map uses a small scale.
charge|The shop will charge a fee.|The soldiers began to charge forward.
match|The teams played a close match.|Strike a match to light the candle.
date|Their first date was at a cafe.|Check the date on the carton.
jam|Traffic was stuck in a jam.|She spread jam on the toast.
clip|Use a clip to hold the papers.|They watched a short film clip.
board|The board approved the budget.|He carried a wooden board upstairs.
bat|A bat flew out at dusk.|She swung the bat at the ball.
mouse|A mouse hid behind the cupboard.|The computer mouse stopped working.
key|She lost the key to the door.|The song is in a minor key.
park|They walked through the park.|Please park beside the gate.
ring|Her ring slipped off her finger.|The phone began to ring.
bridge|They crossed the bridge on foot.|The dentist fitted a bridge.
screen|The phone screen cracked.|They used trees to screen the house.
plot|The novel's plot was complex.|They bought a small plot of land.
record|The runner broke the record.|Please record the meeting.
present|She received a birthday present.|He will present the findings tomorrow.
object|The museum displayed a rare object.|I object to the proposed fee.
minute|Wait a minute before opening it.|A minute detail changed the result.
content|The article's content was useful.|She felt content with her choice.
desert|They crossed the desert at dawn.|Do not desert your teammates.
tear|A tear rolled down her cheek.|Try not to tear the paper.
lead|The guide will lead the group.|The old pipes contain lead.
bow|She tied a bow on the gift.|The actor took a bow after the show.
row|They sat in the front row.|The neighbours had a row over noise.
well|The village drew water from a well.|She performed well in the exam.
right|Turn right at the corner.|Everyone has a right to appeal.
left|He left the room quietly.|Turn left after the bridge.
watch|I checked my watch for the time.|Watch the children near the pool.
train|The train arrived on time.|They train together twice a week.
file|She saved the file on her laptop.|Use a file to smooth the edge.
strike|The workers voted to strike.|A storm could strike the coast.
kind|What kind of bread is this?|She was kind to the newcomer.
sound|The bell made a loud sound.|The bridge is structurally sound.
sharp|This knife has a sharp blade.|She gave a sharp reply.
plain|The valley spread across the plain.|He preferred a plain white shirt.
""",
    "modern_slang": """
gaslighting|She accused him of gaslighting her during arguments.
doomscrolling|He spent the evening doomscrolling through bad news.
rizz|His friends joked that he had no rizz.
delulu|The fan called her own prediction delulu.
ghosting|After three dates, he stopped replying and she called it ghosting.
stan|She has been a stan of the band for years.
sus|That message looks sus to me.
slay|Her friends said she would slay at the audition.
vibe|The cafe has a relaxed vibe.
cringe|The old advert made everyone cringe.
meme|The meme spread across the group chat.
ratio|His post got a ratio after the reply went viral.
FOMO|She booked a ticket because of FOMO.
YOLO|He shouted YOLO before jumping into the pool.
IRL|We had only chatted online before meeting IRL.
DM|Send me a DM with the details.
unfollow|She decided to unfollow the account.
unfriend|He chose to unfriend the stranger.
clickbait|The headline looked like clickbait.
influencer|The influencer promoted a new skincare brand.
livestream|They watched the concert on a livestream.
hashtag|She added a hashtag to the post.
emoji|He ended the message with an emoji.
cancelled|The comedian said he had been cancelled online.
ship|Fans ship the two fictional characters.
flex|He posted the receipt as a flex.
clapback|Her sharp clapback drew thousands of replies.
lowkey|I am lowkey excited about the trip.
highkey|She is highkey obsessed with that show.
mid|The film was mid, according to his review.
based|Several commenters called her take based.
cap|They said his story was cap.
no cap|The concert was incredible, no cap.
salty|He sounded salty after losing the match.
thirsty|The replies looked thirsty rather than friendly.
extra|She said the elaborate apology was extra.
main character energy|He walked into the room with main character energy.
touch grass|His friends told him to touch grass after the argument.
brainrot|She joked that the endless videos had given her brainrot.
skibidi|The children kept repeating skibidi at lunch.
glow up|Her friends admired her glow up.
soft launch|They did a soft launch of their relationship online.
hard launch|The couple made a hard launch with a photo together.
subtweet|He posted a subtweet about the disagreement.
deplatform|The service chose to deplatform the account.
finsta|She shared the photo only on her finsta.
catfish|The profile was a catfish using stolen photos.
ragebait|The outrageous post seemed designed as ragebait.
shitpost|He made a shitpost to amuse his followers.
parasocial|She described the one-sided attachment as parasocial.
""",
    "computing": """
idempotent|The API's idempotent operation can be retried safely.
mutex|The threads used a mutex to guard the shared counter.
polymorphism|The course introduced polymorphism through interface examples.
deterministic|The test expects deterministic output from the parser.
algorithm|The algorithm sorts the list efficiently.
compiler|The compiler rejected the invalid source file.
interpreter|The interpreter executed each statement in order.
runtime|The error appeared only at runtime.
kernel|The kernel handled the system call.
thread|The program started another thread for the task.
process|A background process consumed the memory.
deadlock|Both threads waited forever in a deadlock.
race condition|A race condition made the result unpredictable.
semaphore|The semaphore limited access to three workers.
cache|The cache stored recently requested pages.
latency|Network latency delayed the response.
throughput|The server's throughput dropped under load.
serialization|Serialization converted the object into bytes.
deserialization|Deserialization restored the saved object.
database|The database stored customer records.
index|The query used an index to find the row.
transaction|The transaction committed both updates together.
rollback|A rollback restored the previous state.
schema|The migration changed the database schema.
query|The query returned twenty rows.
endpoint|The mobile app called the login endpoint.
webhook|The webhook notified the service about payment.
payload|The request payload contained a JSON object.
token|The login token expired after an hour.
encryption|Encryption protected the stored file.
hashing|Hashing produced a fixed-size digest.
salt|The password hash included a random salt.
firewall|The firewall blocked the unwanted connection.
container|The service ran inside a container.
virtual machine|They started a virtual machine for testing.
repository|The repository held the source code.
branch|She created a branch for the fix.
merge|The developer resolved a conflict during the merge.
commit|The commit changed three files.
dependency|The project gained a new dependency.
framework|The team built the app with a web framework.
middleware|The middleware checked each request.
recursion|The function used recursion to traverse the tree.
iterator|The iterator returned one item at a time.
pointer|The pointer referred to allocated memory.
buffer|The buffer held data before writing it.
socket|The client opened a socket to the server.
protocol|The protocol defined the message format.
bandwidth|The upload used most of the available bandwidth.
parse|The parser will parse the input string.
""",
    "academic_scientific": """
hypothesis|The experiment tested the hypothesis.
theorem|She proved the theorem in class.
corollary|The corollary followed from the main result.
axiom|The proof began with an axiom.
proof|The mathematician checked each step of the proof.
variable|The researcher changed one variable at a time.
coefficient|The coefficient was negative in the model.
derivative|They calculated the derivative of the function.
integral|The class evaluated the integral.
vector|The vector pointed north on the diagram.
matrix|The matrix had three rows and four columns.
molecule|The molecule contains two oxygen atoms.
atom|The atom lost an electron.
isotope|The isotope decayed over time.
enzyme|The enzyme sped up the reaction.
genome|The team sequenced the organism's genome.
allele|The child inherited one allele from each parent.
phenotype|The phenotype varied with the environment.
photosynthesis|The lesson explained photosynthesis in leaves.
osmosis|Water crossed the membrane through osmosis.
entropy|The physicist calculated the system's entropy.
equilibrium|The reaction reached equilibrium overnight.
catalyst|The catalyst increased the reaction rate.
reagent|The technician added a reagent to the sample.
pH|They measured the water's pH.
quantum|The lecture introduced quantum mechanics.
wavelength|The instrument measured the light's wavelength.
frequency|The signal's frequency increased.
amplitude|The wave's amplitude doubled.
velocity|The object's velocity changed direction.
acceleration|The car's acceleration was measured.
momentum|The collision changed the ball's momentum.
inertia|Inertia kept the object moving.
gravity|Gravity pulled the stone toward Earth.
ecosystem|The wetland ecosystem supports many birds.
biodiversity|The survey measured forest biodiversity.
taxonomy|The biologist revised the species taxonomy.
species|The survey found a new species of moth.
population|The study sampled the island population.
sample|The lab tested a sample of water.
control group|The control group received no treatment.
placebo|The trial compared the drug with a placebo.
longitudinal|The longitudinal study followed children for years.
correlation|The chart showed a correlation between the variables.
causation|The researchers warned that correlation does not prove causation.
confidence interval|The estimate included a confidence interval.
p-value|The paper reported a p-value for the test.
peer review|The manuscript entered peer review.
methodology|The paper described its methodology in detail.
reproducibility|The lab tested the result's reproducibility.
""",
    "idioms": """
break the ice|She told a joke to break the ice.
take for granted|Do not take for granted the help you receive.
rule of thumb|As a rule of thumb, allow an hour for the trip.
bite the bullet|They decided to bite the bullet and pay for repairs.
hit the nail on the head|Your comment hit the nail on the head.
once in a blue moon|We eat out once in a blue moon.
under the weather|He stayed home because he felt under the weather.
spill the beans|Please do not spill the beans about the party.
cost an arm and a leg|The repairs could cost an arm and a leg.
a piece of cake|The quiz was a piece of cake for her.
let the cat out of the bag|He let the cat out of the bag before the announcement.
burn the midnight oil|She had to burn the midnight oil before the deadline.
beat around the bush|Stop trying to beat around the bush and tell me.
on the same page|We checked that everyone was on the same page.
the ball is in your court|I sent the proposal, so the ball is in your court.
cut corners|The contractor did not cut corners on safety.
get cold feet|She began to get cold feet before the wedding.
go the extra mile|The nurse chose to go the extra mile for patients.
in hot water|He was in hot water after missing the deadline.
jump the gun|Do not jump the gun before the results arrive.
keep an eye on|Please keep an eye on the oven.
miss the boat|Apply today so you do not miss the boat.
pull someone's leg|He tried to pull someone's leg with the story.
see eye to eye|The colleagues did not see eye to eye on the budget.
sit on the fence|She refused to sit on the fence during the vote.
take the plunge|They chose to take the plunge and open a shop.
through thick and thin|They stayed friends through thick and thin.
up in the air|The travel plans are still up in the air.
when pigs fly|He said he would volunteer when pigs fly.
at the drop of a hat|She can leave at the drop of a hat.
back to square one|The failed test sent us back to square one.
bend over backwards|The staff will bend over backwards to help.
by the book|The inspector did everything by the book.
call it a day|After six hours, they decided to call it a day.
cross that bridge when we come to it|We can cross that bridge when we come to it.
down to earth|Her down to earth advice was useful.
face the music|He had to face the music after the mistake.
get the hang of it|You will get the hang of it with practice.
give someone the benefit of the doubt|Try to give someone the benefit of the doubt.
have a chip on your shoulder|You seem to have a chip on your shoulder.
in the nick of time|The ambulance arrived in the nick of time.
keep your chin up|Keep your chin up while you recover.
leave no stone unturned|The team promised to leave no stone unturned.
make ends meet|They worked two jobs to make ends meet.
no strings attached|The grant came with no strings attached.
out of the blue|She called me out of the blue.
play it by ear|We can play it by ear if the weather changes.
put all your eggs in one basket|Do not put all your eggs in one basket.
the last straw|The third delay was the last straw.
turn a blind eye|The manager would not turn a blind eye to fraud.
""",
    "phrasal_verbs": """
give up|She refused to give up after failing once.
look up|Please look up the address online.
turn down|He chose to turn down the offer.
put off|They had to put off the meeting.
carry on|Please carry on with the lesson.
break down|The old car might break down.
bring up|I will bring up the issue tomorrow.
call off|The organisers may call off the match.
check in|Guests can check in after three.
check out|We need to check out before noon.
come across|You may come across an unfamiliar word.
come up with|Can you come up with another plan?
cut down on|The doctor told him to cut down on sugar.
drop off|Please drop off the parcel at reception.
fill in|Could you fill in this form?
find out|We will find out the result tonight.
get along|The siblings usually get along well.
get away|They hope to get away for the weekend.
get over|It took months to get over the illness.
go on|Please go on with your story.
grow up|She wants to grow up near the sea.
hand in|Students must hand in the assignment.
hang out|They often hang out after school.
hold on|Please hold on while I check.
keep up|He struggled to keep up with the class.
let down|I do not want to let down the team.
look after|She will look after the dog.
look for|We need to look for the missing key.
look into|The council will look into the complaint.
make up|The children like to make up stories.
move on|It was time to move on after the setback.
pass out|He felt faint and thought he might pass out.
pick up|Can you pick up the children at four?
point out|She tried to point out the error.
put away|Please put away the clean dishes.
put up with|I cannot put up with the noise.
run into|We may run into traffic on the way.
set off|They plan to set off at dawn.
show up|He promised to show up for the interview.
sort out|They need to sort out the billing problem.
take off|The plane will take off in ten minutes.
take over|A new manager will take over next week.
throw away|Do not throw away the receipt.
turn around|The driver had to turn around.
turn on|Please turn on the lamp.
turn off|Remember to turn off the stove.
wake up|The alarm will wake up the children.
work out|We can work out the total together.
write down|Please write down your phone number.
back up|Remember to back up the files.
""",
    "inflected_irregular": """
ran|She ran to catch the bus.
went|He went home after work.
children|The children played outside.
better|Her health is better this week.
brought|She brought the books to class.
ate|They ate dinner early.
drank|He drank a glass of water.
drove|She drove through the city.
flew|The bird flew over the roof.
forgot|I forgot my umbrella.
gave|He gave her a notebook.
grew|The tree grew taller each year.
heard|She heard a knock at the door.
held|He held the rope firmly.
kept|They kept the receipt.
knew|She knew the answer already.
lost|He lost the house keys.
made|They made soup for dinner.
met|I met her at the station.
paid|She paid the invoice yesterday.
rode|The child rode a bicycle to school.
rose|The water rose after the storm.
saw|She saw a fox near the road.
sent|He sent the package on Monday.
shook|The earthquake shook the building.
sang|The choir sang quietly.
slept|The baby slept through the night.
spoke|She spoke to the manager.
spent|They spent the afternoon gardening.
stood|He stood beside the gate.
swam|She swam across the pool.
taught|He taught science for ten years.
took|She took the last seat.
thought|I thought about your suggestion.
threw|He threw the ball over the fence.
wore|She wore a blue coat.
wrote|He wrote a letter to his friend.
broken|The broken cup lay on the floor.
chosen|The chosen candidate accepted the job.
done|The work was done before noon.
fallen|The fallen leaves covered the path.
forgotten|The forgotten keys were still on the table.
frozen|The frozen lake was unsafe to cross.
hidden|The hidden door opened slowly.
written|The written instructions were clear.
mice|The mice ran into the barn.
geese|The geese flew south.
feet|Her feet were wet after the walk.
teeth|He brushed his teeth before bed.
women|The women gathered for the meeting.
""",
    "regional": """
jandals|She wore jandals to the beach.
bach|They stayed in a bach by the lake.
dairy|He bought milk at the corner dairy.
chilly bin|The drinks stayed cold in the chilly bin.
tramping|They went tramping in the mountains.
togs|Bring your togs for the swim.
heaps|There were heaps of people at the concert.
sweet as|The mechanic said the car was sweet as.
hard yakka|Digging the trench was hard yakka.
pōwhiri|The visitors attended a pōwhiri at the marae.
whānau|Her whānau gathered for dinner.
kai|They shared kai after the meeting.
kūmara|They roasted kūmara with the vegetables.
marae|The group met at the marae.
mana|The leader earned mana through service.
puku|The child patted his full puku.
pāua|They found a pāua shell on the shore.
tuatara|A tuatara rested in the sanctuary.
kea|A kea perched near the trail.
wētā|A wētā hid under the log.
munted|The bicycle was munted after the crash.
chur|He said chur when his friend helped.
yeah nah|Her reply was yeah nah, so we waited.
she'll be right|He said she'll be right despite the rain.
arvo|We will meet this arvo.
brekkie|He ate brekkie before work.
sunnies|She put on her sunnies outside.
torch|Take a torch for the dark path.
boot|The groceries are in the car boot.
bonnet|The mechanic opened the car bonnet.
flat|They rented a small flat near the station.
lift|We took the lift to the fourth floor.
lorry|A lorry blocked the narrow street.
biscuit|She had a biscuit with her tea.
crisps|He bought a packet of crisps.
chips|They served fish and chips for lunch.
courgette|She sliced a courgette for the stew.
aubergine|The recipe calls for one aubergine.
nappy|The baby needs a clean nappy.
dummy|The toddler dropped his dummy.
queue|We waited in a queue outside.
jumper|He wore a woollen jumper in winter.
trainers|She bought new trainers for running.
wellies|The children wore wellies in the mud.
loo|The sign pointed toward the loo.
ta|He said ta after receiving the parcel.
knackered|She was knackered after the shift.
gobsmacked|He was gobsmacked by the result.
bloody|It was a bloody nuisance to repair.
cheers|She said cheers when the driver stopped.
""",
}

CHALLENGES = {
    "ephemeral", "ubiquitous", "pragmatic", "ostensibly", "nuanced", "pedantic",
    "run", "set", "bank", "fine", "light", "sanction", "gaslighting",
    "doomscrolling", "rizz", "delulu", "idempotent", "mutex",
    "polymorphism", "deterministic", "break the ice", "take for granted",
    "rule of thumb", "ran", "went", "children", "better", "brought",
}


def build() -> list[dict[str, str]]:
    records = []
    for category, block in GROUPS.items():
        items = []
        for line in block.strip().splitlines():
            fields = [field.strip() for field in line.split("|")]
            if len(fields) not in (2, 3):
                raise ValueError(f"Invalid corpus row: {line}")
            term, context_1 = fields[:2]
            context_2 = fields[2] if len(fields) == 3 else ""
            if term.casefold() not in context_1.casefold() or (context_2 and term.casefold() not in context_2.casefold()):
                raise ValueError(f"Term missing from its context: {term}")
            items.append({"term": term, "category": category, "context_1": context_1,
                          "context_2": context_2,
                          "selection_tag": "named_challenge" if term.casefold() in CHALLENGES else "curated"})
        if len(items) != 50:
            raise ValueError(f"Expected 50 {category} items; got {len(items)}")
        if category == "polysemous" and any(not item["context_2"] for item in items):
            raise ValueError("Every polysemous item needs two contexts")
        records.extend(items)
    terms = [record["term"].casefold() for record in records]
    if len(records) != 500 or len(set(terms)) != 500:
        from collections import Counter
        raise ValueError(f"Corpus must have 500 distinct terms: {[term for term, count in Counter(terms).items() if count > 1]}")
    if not CHALLENGES <= set(terms):
        raise ValueError(f"Missing named challenges: {CHALLENGES - set(terms)}")
    random.Random(SEED).shuffle(records)
    for number, record in enumerate(records, 1):
        record["id"] = f"B{number:04d}"
    return records


def main() -> None:
    records = build()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=["id", "term", "category", "context_1", "context_2", "selection_tag"])
        writer.writeheader()
        writer.writerows(records)
    print(f"Wrote {len(records)} items to {OUTPUT}")


if __name__ == "__main__":
    main()

